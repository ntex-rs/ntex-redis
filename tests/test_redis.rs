use ntex::util::{Bytes, HashMap};
use ntex::{service::Pipeline, util::stream_recv};
use ntex_redis::{Client, RedisConnector, array, cmd, codec::Response, errors::ConnectError};
use rand::{Rng, distr::Alphanumeric, rng};
use std::time::{Duration, SystemTime};

async fn connect() -> Client {
    RedisConnector::new("127.0.0.1:6379")
        .connect()
        .await
        .unwrap()
}

fn new_key() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

#[ntex::test]
async fn test_auth() {
    let result = RedisConnector::new("127.0.0.1:6379")
        .password("test")
        .connect()
        .await;
    assert!(matches!(result, Err(ConnectError::Unauthorized)));
}

#[ntex::test]
async fn test_auth_simple() {
    let result = RedisConnector::new("127.0.0.1:6379")
        .password("test")
        .connect_simple()
        .await;
    assert!(matches!(result, Err(ConnectError::Unauthorized)));
}

#[ntex::test]
async fn test_strings() {
    let _ = env_logger::try_init();
    let redis = connect().await;
    let key = new_key();
    let result = redis.exec(cmd::Set(&key, "1")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Get(&key)).await.unwrap().unwrap();
    assert_eq!(resp, "1");

    let resp = redis.exec(cmd::IncrBy(&key, 10)).await.unwrap();
    assert_eq!(resp, 11);

    let resp = redis.exec(cmd::Get("unknown")).await.unwrap();
    assert_eq!(resp, None);
}

#[ntex::test]
async fn test_keys() {
    let redis = connect().await;
    let key1 = new_key();
    let key2 = new_key();

    let result = redis.exec(cmd::Set(&key1, "value")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Exists(&key1)).await.unwrap();
    assert_eq!(resp, 1);
    let resp = redis.exec(cmd::Exists(&key2)).await.unwrap();
    assert_eq!(resp, 0);
    let resp = redis.exec(cmd::Exists(&key1).key(&key2)).await.unwrap();
    assert_eq!(resp, 1);

    let resp = redis.exec(cmd::Ttl(&key1)).await.unwrap();
    assert_eq!(resp, cmd::TtlResult::NoExpire);
    let resp = redis.exec(cmd::Expire(&key1, 1)).await.unwrap();
    assert!(resp);
    let resp = redis.exec(cmd::Ttl(&key1)).await.unwrap();
    assert_eq!(resp, cmd::TtlResult::Seconds(1));
    // unix time of now() + 10sec
    let expire_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        + Duration::from_millis(10350);
    let resp = redis
        .exec(cmd::ExpireAt(&key1, expire_at.as_secs() as i64))
        .await
        .unwrap();
    assert!(resp);
    let resp = redis.exec(cmd::Ttl(&key1)).await.unwrap();
    // expire time is truncated to whole seconds, remaining ttl is 9.35..10.35s
    assert!(matches!(resp, cmd::TtlResult::Seconds(9..=10)), "{resp:?}");
    let resp = redis.exec(cmd::Ttl(&key2)).await.unwrap();
    assert_eq!(resp, cmd::TtlResult::NotFound);
    let resp = redis.exec(cmd::Expire(&key2, 1)).await.unwrap();
    assert!(!resp);

    let resp = redis
        .exec(cmd::Del(&key1).keys(vec![&key2, &key1]))
        .await
        .unwrap();
    assert_eq!(resp, 1);
    let resp = redis.exec(cmd::Exists(&key1)).await.unwrap();
    assert_eq!(resp, 0);
}

#[ntex::test]
async fn test_strings_simple() {
    let redis = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await
        .unwrap();
    let key = new_key();

    let result = redis.exec(cmd::Set(&key, "value")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Get(&key)).await.unwrap().unwrap();
    assert_eq!(resp, "value");

    let resp = redis.exec(cmd::Get("unknown")).await.unwrap();
    assert_eq!(resp, None);
}

#[ntex::test]
async fn test_lists() {
    let redis = connect().await;
    let key = new_key();

    let result = redis
        .exec(cmd::LPush(&key, "l_value").if_exists())
        .await
        .unwrap();
    assert_eq!(result, 0);
    let result = redis
        .exec(cmd::RPush(&key, "l_value").if_exists())
        .await
        .unwrap();
    assert_eq!(result, 0);

    let result = redis.exec(cmd::LPush(&key, "l_value")).await.unwrap();
    assert_eq!(result, 1);
    let result = redis.exec(cmd::RPush(&key, "r_value")).await.unwrap();
    assert_eq!(result, 2);

    let resp = redis.exec(cmd::LIndex(&key, 0)).await.unwrap().unwrap();
    assert_eq!(resp, "l_value");
    let resp = redis.exec(cmd::LIndex(&key, 1)).await.unwrap().unwrap();
    assert_eq!(resp, "r_value");

    let resp = redis.exec(cmd::LPop(&key)).await.unwrap();
    assert_eq!(resp.unwrap(), "l_value");
    let resp = redis.exec(cmd::RPop(&key)).await.unwrap();
    assert_eq!(resp.unwrap(), "r_value");
    let resp = redis.exec(cmd::LPop(&key)).await.unwrap();
    assert_eq!(resp, None);
}

#[ntex::test]
async fn test_hashes() {
    let redis = connect().await;
    let key = new_key();

    let result = redis
        .exec(cmd::HSet(&key, "field1", "1").entry("field2", "2"))
        .await
        .unwrap();
    assert_eq!(result, 2);

    let result = redis.exec(cmd::HIncrBy(&key, "field1", 10)).await.unwrap();
    assert_eq!(result, 11);

    let result = redis.exec(cmd::HGet(&key, "field1")).await.unwrap();
    assert_eq!(result.unwrap(), "11");

    let result = redis.exec(cmd::HGetAll(&key)).await.unwrap();
    let mut expected = HashMap::default();
    expected.insert("field1".into(), "11".into());
    expected.insert("field2".into(), "2".into());
    assert_eq!(result, expected);

    let result = redis
        .exec(cmd::HDel(&key, "field1").remove("field2"))
        .await
        .unwrap();
    assert_eq!(result, 2);

    let result = redis.exec(cmd::HGet(&key, "field1")).await.unwrap();
    assert_eq!(result, None);

    let result = redis.exec(cmd::HGetAll(&key)).await.unwrap();
    assert!(result.is_empty());
}

#[ntex::test]
async fn test_connection() {
    let redis = connect().await;

    let result = redis.exec(cmd::Ping()).await.unwrap();
    assert_eq!(result, "PONG");

    let result = redis.exec(cmd::Select(1)).await.unwrap();
    assert!(result);
}

#[ntex::test]
async fn test_flushdb() {
    let redis = connect().await;
    assert!(redis.exec(cmd::Select(15)).await.unwrap());

    let key = new_key();
    redis.exec(cmd::Set(&key, "value")).await.unwrap();
    redis.flushdb().await.unwrap();

    // client stays in sync after flushdb
    let result = redis.exec(cmd::Get(&key)).await.unwrap();
    assert_eq!(result, None);
    let result = redis.exec(cmd::Ping()).await.unwrap();
    assert_eq!(result, "PONG");
}

#[ntex::test]
async fn test_subscribe() {
    let key = new_key();
    let channel = Bytes::from(key);

    let subscriber = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await
        .unwrap();

    // sub
    let pubsub = subscriber
        .subscribe(cmd::Subscribe(vec![&channel]))
        .unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Subscribed(channel.clone())
    );

    let publisher = connect().await;

    // pub
    let result = publisher.exec(cmd::Publish(&channel, "1")).await.unwrap();
    assert_eq!(result, 1);

    // receive message
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Message {
            pattern: None,
            channel: channel.clone(),
            payload: Bytes::from_static(b"1")
        }
    );

    // unsub
    pubsub.send(cmd::UnSubscribe(Some(vec![&channel]))).unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::UnSubscribed(channel.clone())
    );

    // unsub from all channels
    pubsub.send(cmd::Subscribe(vec![&channel])).unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Subscribed(channel.clone())
    );
    pubsub.send(cmd::UnSubscribe::<&str>(None)).unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::UnSubscribed(channel.clone())
    );

    // back to client state
    let client = pubsub.into_client();
    client.exec(cmd::Reset()).await.unwrap();
}

#[ntex::test]
async fn test_ssubscribe() {
    let key = new_key();
    let channel = Bytes::from(key);

    let subscriber = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await
        .unwrap();

    // sub
    let pubsub = subscriber
        .subscribe(cmd::SSubscribe(vec![&channel]))
        .unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Subscribed(channel.clone())
    );

    let publisher = connect().await;

    // pub
    let result = publisher.exec(cmd::SPublish(&channel, "1")).await.unwrap();
    assert_eq!(result, 1);

    // receive message
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Message {
            pattern: None,
            channel: channel.clone(),
            payload: Bytes::from_static(b"1")
        }
    );

    // unsub
    pubsub
        .send(cmd::SUnSubscribe(Some(vec![&channel])))
        .unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::UnSubscribed(channel.clone())
    );
}

#[ntex::test]
async fn test_psubscribe() {
    let part = new_key();
    let channel = Bytes::from(format!("channel:{}", part));
    let pattern = Bytes::from("channel:*");

    let subscriber = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await
        .unwrap();

    // sub
    let pubsub = subscriber
        .subscribe(cmd::PSubscribe(vec![&pattern]))
        .unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Subscribed(pattern.clone()),
    );

    let publisher = connect().await;

    // pub
    let result = publisher.exec(cmd::Publish(&channel, "1")).await.unwrap();
    assert_eq!(result, 1);

    // receive message
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::Message {
            pattern: Some(pattern.clone()),
            channel: channel.clone(),
            payload: Bytes::from_static(b"1")
        }
    );

    // unsub
    pubsub
        .send(cmd::PUnSubscribe(Some(vec![&pattern])))
        .unwrap();
    let message = pubsub.recv().await;
    assert_eq!(
        message.unwrap().unwrap(),
        cmd::SubscribeItem::UnSubscribed(pattern.clone())
    );
}

#[ntex::test]
async fn test_set_options() {
    let redis = connect().await;
    let key = new_key();

    // XX on missing key
    let result = redis.exec(cmd::Set(&key, "1").if_exists()).await.unwrap();
    assert!(!result);
    assert_eq!(redis.exec(cmd::Get(&key)).await.unwrap(), None);

    // NX on missing key
    let result = redis
        .exec(cmd::Set(&key, "1").if_not_exists())
        .await
        .unwrap();
    assert!(result);

    // NX on existing key
    let result = redis
        .exec(cmd::Set(&key, "2").if_not_exists())
        .await
        .unwrap();
    assert!(!result);
    assert_eq!(redis.exec(cmd::Get(&key)).await.unwrap().unwrap(), "1");

    // EX
    let result = redis
        .exec(cmd::Set(&key, "3").if_exists().expire_secs(100))
        .await
        .unwrap();
    assert!(result);
    let ttl = redis.exec(cmd::Ttl(&key)).await.unwrap();
    assert!(matches!(ttl, cmd::TtlResult::Seconds(99..=100)), "{ttl:?}");

    // KEEPTTL
    let result = redis.exec(cmd::Set(&key, "4").keepttl()).await.unwrap();
    assert!(result);
    let ttl = redis.exec(cmd::Ttl(&key)).await.unwrap();
    assert!(matches!(ttl, cmd::TtlResult::Seconds(99..=100)), "{ttl:?}");

    // PX
    let result = redis
        .exec(cmd::Set(&key, "5").expire_millis(50_000))
        .await
        .unwrap();
    assert!(result);
    let ttl = redis.exec(cmd::Ttl(&key)).await.unwrap();
    assert!(matches!(ttl, cmd::TtlResult::Seconds(49..=50)), "{ttl:?}");

    // SET without KEEPTTL clears ttl
    redis.exec(cmd::Set(&key, "6")).await.unwrap();
    let ttl = redis.exec(cmd::Ttl(&key)).await.unwrap();
    assert_eq!(ttl, cmd::TtlResult::NoExpire);

    redis.exec(cmd::Del(&key)).await.unwrap();
}

#[ntex::test]
async fn test_keys_pattern() {
    let redis = connect().await;
    let prefix = new_key();
    let key1 = format!("{prefix}:1");
    let key2 = format!("{prefix}:2");
    redis.exec(cmd::Set(&key1, "1")).await.unwrap();
    redis.exec(cmd::Set(&key2, "2")).await.unwrap();

    let mut keys = redis.exec(cmd::Keys(format!("{prefix}:*"))).await.unwrap();
    keys.sort();
    assert_eq!(keys, [key1.as_str(), key2.as_str()]);

    let keys = redis.exec(cmd::Keys(format!("{prefix}:x*"))).await.unwrap();
    assert!(keys.is_empty());

    let result = redis.exec(cmd::Del(&key1).key(&key2)).await.unwrap();
    assert_eq!(result, 2);
}

#[ntex::test]
async fn test_lists_extend() {
    let redis = connect().await;
    let key = new_key();

    let result = redis
        .exec(cmd::RPush(&key, "1").extend(["2", "3"]))
        .await
        .unwrap();
    assert_eq!(result, 3);
    let result = redis
        .exec(cmd::LPush(&key, "0").value("-1").if_exists())
        .await
        .unwrap();
    assert_eq!(result, 5);

    let result = redis.exec(cmd::LIndex(&key, 0)).await.unwrap().unwrap();
    assert_eq!(result, "-1");
    let result = redis.exec(cmd::RPop(&key)).await.unwrap().unwrap();
    assert_eq!(result, "3");

    redis.exec(cmd::Del(&key)).await.unwrap();
}

#[ntex::test]
async fn test_hashes_remove_all() {
    let redis = connect().await;
    let key = new_key();

    redis
        .exec(cmd::HSet(&key, "f1", "1").entry("f2", "2").entry("f3", "3"))
        .await
        .unwrap();
    let result = redis
        .exec(cmd::HDel(&key, "f1").remove_all(["f2", "unknown"]))
        .await
        .unwrap();
    assert_eq!(result, 2);
    assert_eq!(redis.exec(cmd::HLen(&key)).await.unwrap(), 1);

    redis.exec(cmd::Del(&key)).await.unwrap();
}

#[ntex::test]
async fn test_client_service() {
    let redis = connect().await;
    let key = new_key();

    let pl = Pipeline::new((), redis.clone());
    pl.ready().await.unwrap();

    let result = pl.call(array!["SET", &key, "value"]).await.unwrap();
    assert_eq!(result, Response::String("OK".into()));
    let result = pl.call(array!["GET", &key]).await.unwrap();
    assert_eq!(result, Response::Bytes(Bytes::from_static(b"value")));
    let result = pl.call(array!["DEL", &key]).await.unwrap();
    assert_eq!(result, Response::Integer(1));

    // error reply
    let result = pl.call(array!["INCR"]).await.unwrap();
    assert!(matches!(result, Response::Error(_)), "{result:?}");
    assert_eq!(redis.exec(cmd::Ping()).await.unwrap(), "PONG");
}

#[ntex::test]
async fn test_connector_config() {
    let redis = RedisConnector::new("127.0.0.1:6379")
        .config(ntex::SharedCfg::default())
        .connector(ntex::connect::Connector::default())
        .connect()
        .await
        .unwrap();
    assert!(redis.is_connected());
    assert_eq!(redis.exec(cmd::Ping()).await.unwrap(), "PONG");
}

#[ntex::test]
async fn test_subscribe_stream() {
    let channel = new_key();

    let subscriber = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await
        .unwrap();
    let mut pubsub = subscriber
        .subscribe(cmd::Subscribe(vec![&channel]))
        .unwrap();
    assert_eq!(
        stream_recv(&mut pubsub).await.unwrap().unwrap(),
        cmd::SubscribeItem::Subscribed(channel.clone().into())
    );

    let publisher = connect().await;
    publisher.exec(cmd::Publish(&channel, "1")).await.unwrap();
    publisher.exec(cmd::Publish(&channel, "2")).await.unwrap();

    assert_eq!(
        stream_recv(&mut pubsub).await.unwrap().unwrap(),
        cmd::SubscribeItem::Message {
            pattern: None,
            channel: channel.clone().into(),
            payload: Bytes::from_static(b"1")
        }
    );
    let item = std::future::poll_fn(|cx| pubsub.poll_recv(cx)).await;
    assert_eq!(
        item.unwrap().unwrap(),
        cmd::SubscribeItem::Message {
            pattern: None,
            channel: channel.clone().into(),
            payload: Bytes::from_static(b"2")
        }
    );
}
