//! Client tests against a scripted mock redis server
use std::io::{Read, Write};
use std::{net::TcpListener, sync::mpsc, thread, time::Duration};

use ntex::{codec::Decoder, time::sleep, util::BytesMut, util::join};
use ntex_redis::codec::{Codec, Response};
use ntex_redis::errors::{CommandError, ConnectError, Error};
use ntex_redis::{RedisConnector, cmd};

enum Step {
    /// Wait for a request and send reply
    Reply(&'static [u8]),
    /// Send data without waiting for a request
    Push(&'static [u8]),
    /// Wait for a request and close connection
    Close,
}

/// Start mock server, returns server address and received requests
fn mock(steps: Vec<Step>) -> (String, mpsc::Receiver<Response>) {
    let lst = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = lst.local_addr().unwrap().to_string();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = lst.accept().unwrap();
        let codec = Codec::new();
        let mut buf = BytesMut::new();
        let mut chunk = [0u8; 1024];

        let mut recv = |stream: &mut std::net::TcpStream| {
            loop {
                if let Some(req) = codec.decode(&mut buf).unwrap() {
                    let _ = tx.send(req);
                    return true;
                }
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => return false,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
        };

        for step in steps {
            match step {
                Step::Reply(data) => {
                    if !recv(&mut stream) {
                        return;
                    }
                    stream.write_all(data).unwrap();
                }
                Step::Push(data) => stream.write_all(data).unwrap(),
                Step::Close => {
                    recv(&mut stream);
                    return;
                }
            }
        }
        // keep connection open until client closes it
        while recv(&mut stream) {}
    });
    (addr, rx)
}

fn request(args: &[&str]) -> Response {
    Response::Array(
        args.iter()
            .map(|a| Response::Bytes(a.as_bytes().to_vec().into()))
            .collect(),
    )
}

fn is_peer_gone(err: &CommandError) -> bool {
    matches!(err, CommandError::Protocol(Error::PeerGone(_)))
}

#[ntex::test]
async fn test_auth_passwords() {
    let (addr, rx) = mock(vec![
        Step::Reply(b"-WRONGPASS invalid password\r\n"),
        Step::Reply(b"+OK\r\n"),
        Step::Reply(b"+PONG\r\n"),
    ]);
    let client = RedisConnector::new(addr)
        .password("first")
        .password("second")
        .password("third")
        .connect()
        .await
        .unwrap();
    assert_eq!(client.exec(cmd::Ping()).await.unwrap(), "PONG");

    assert_eq!(rx.recv().unwrap(), request(&["AUTH", "first"]));
    assert_eq!(rx.recv().unwrap(), request(&["AUTH", "second"]));
    assert_eq!(rx.recv().unwrap(), request(&["PING"]));
}

#[ntex::test]
async fn test_auth_unauthorized() {
    let (addr, _rx) = mock(vec![
        Step::Reply(b"-WRONGPASS invalid password\r\n"),
        Step::Reply(b"+NOTOK\r\n"),
    ]);
    let res = RedisConnector::new(addr)
        .password("first")
        .password("second")
        .connect_simple()
        .await;
    assert!(matches!(res, Err(ConnectError::Unauthorized)));
}

#[ntex::test]
async fn test_auth_peer_gone() {
    let (addr, _rx) = mock(vec![Step::Close]);
    let res = RedisConnector::new(addr)
        .password("first")
        .password("second")
        .connect()
        .await;
    match res {
        Err(ConnectError::Command(err)) => assert!(is_peer_gone(&err), "{err:?}"),
        res => panic!("{res:?}"),
    }
}

#[ntex::test]
async fn test_connect_error() {
    let addr = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    let res = RedisConnector::new(addr).connect().await;
    assert!(matches!(res, Err(ConnectError::Connect(_))), "{res:?}");
}

#[ntex::test]
async fn test_client_peer_gone() {
    let (addr, _rx) = mock(vec![Step::Reply(b"+PONG\r\n"), Step::Close]);
    let client = RedisConnector::new(addr).connect().await.unwrap();
    assert!(client.is_connected());
    assert_eq!(client.exec(cmd::Ping()).await.unwrap(), "PONG");

    let err = client.exec(cmd::Ping()).await.unwrap_err();
    assert!(is_peer_gone(&err), "{err:?}");

    sleep(Duration::from_millis(50)).await;
    assert!(!client.is_connected());
    assert!(format!("{client:?}").contains("connected: false"));
    let err = client.exec(cmd::Ping()).await.unwrap_err();
    assert!(is_peer_gone(&err), "{err:?}");
    assert!(client.flushdb().await.is_err());
}

#[ntex::test]
async fn test_client_decode_error() {
    let (addr, _rx) = mock(vec![Step::Reply(b"!bad\r\n")]);
    let client = RedisConnector::new(addr).connect().await.unwrap();

    // in-flight requests fail after protocol error
    let (res1, res2) = join(client.exec(cmd::Ping()), client.exec(cmd::Ping())).await;
    assert!(
        matches!(res1, Err(CommandError::Protocol(Error::Parse(_)))),
        "{res1:?}"
    );
    assert!(is_peer_gone(&res2.unwrap_err()));

    sleep(Duration::from_millis(50)).await;
    assert!(!client.is_connected());
}

#[ntex::test]
async fn test_client_unexpected_response() {
    let (addr, _rx) = mock(vec![Step::Push(b":1\r\n"), Step::Reply(b"+PONG\r\n")]);
    let client = RedisConnector::new(addr).connect().await.unwrap();

    // unexpected response is dropped
    sleep(Duration::from_millis(50)).await;
    assert_eq!(client.exec(cmd::Ping()).await.unwrap(), "PONG");
}

#[ntex::test]
async fn test_client_large_response() {
    let mut data = b"$4000000\r\n".to_vec();
    data.extend_from_slice(&[b'x'; 4_000_000]);
    data.extend_from_slice(b"\r\n");
    let (addr, _rx) = mock(vec![Step::Reply(data.leak())]);
    let client = RedisConnector::new(addr).connect().await.unwrap();

    let value = client.exec(cmd::Get("key")).await.unwrap().unwrap();
    assert_eq!(value.len(), 4_000_000);
    assert!(value.iter().all(|b| *b == b'x'));
}

#[ntex::test]
async fn test_simple_errors() {
    // protocol error
    let (addr, _rx) = mock(vec![Step::Reply(b"!bad\r\n")]);
    let client = RedisConnector::new(addr).connect_simple().await.unwrap();
    let res = client.exec(cmd::Ping()).await;
    assert!(
        matches!(res, Err(CommandError::Protocol(Error::Parse(_)))),
        "{res:?}"
    );

    // connection closed
    let (addr, _rx) = mock(vec![Step::Close]);
    let client = RedisConnector::new(addr).connect_simple().await.unwrap();
    let err = client.exec(cmd::Ping()).await.unwrap_err();
    assert!(is_peer_gone(&err), "{err:?}");
}

#[ntex::test]
async fn test_subscription_errors() {
    let (addr, _rx) = mock(vec![
        Step::Reply(b"*3\r\n$9\r\nsubscribe\r\n$2\r\nch\r\n:1\r\n"),
        Step::Push(b"*3\r\n$7\r\nunknown\r\n$2\r\nch\r\n$1\r\nx\r\n"),
        Step::Push(b"*2\r\n$7\r\nmessage\r\n$2\r\nch\r\n"),
        Step::Push(b"-ERR failed\r\n"),
    ]);
    let client = RedisConnector::new(addr).connect_simple().await.unwrap();
    let pubsub = client.subscribe(cmd::Subscribe(vec!["ch"])).unwrap();
    assert_eq!(
        pubsub.recv().await.unwrap().unwrap(),
        cmd::SubscribeItem::Subscribed("ch".into())
    );
    assert!(matches!(
        pubsub.recv().await.unwrap(),
        Err(CommandError::Output("Subscription message type unknown", _))
    ));
    assert!(matches!(
        pubsub.recv().await.unwrap(),
        Err(CommandError::Output(..))
    ));
    assert!(matches!(
        pubsub.recv().await.unwrap(),
        Err(CommandError::Error(e)) if e == "ERR failed"
    ));
}
