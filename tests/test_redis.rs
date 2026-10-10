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

#[ntex::test]
async fn test_strings_commands() {
    let redis = connect().await;
    let key = new_key();

    assert_eq!(redis.exec(cmd::Incr(&key)).await.unwrap(), 1);
    assert_eq!(redis.exec(cmd::Decr(&key)).await.unwrap(), 0);
    assert_eq!(redis.exec(cmd::DecrBy(&key, 5)).await.unwrap(), -5);
    assert_eq!(redis.exec(cmd::IncrByFloat(&key, 1.5)).await.unwrap(), -3.5);
    assert!(redis.exec(cmd::Incr(&key)).await.is_err());
    redis.exec(cmd::Del(&key)).await.unwrap();

    assert_eq!(redis.exec(cmd::Append(&key, "Hello")).await.unwrap(), 5);
    assert_eq!(redis.exec(cmd::Append(&key, " World")).await.unwrap(), 11);
    assert_eq!(redis.exec(cmd::StrLen(&key)).await.unwrap(), 11);
    assert_eq!(
        redis.exec(cmd::GetRange(&key, 0, 4)).await.unwrap(),
        "Hello"
    );
    assert_eq!(
        redis.exec(cmd::GetRange(&key, -5, -1)).await.unwrap(),
        "World"
    );
    assert_eq!(
        redis.exec(cmd::SetRange(&key, 6, "Redis")).await.unwrap(),
        11
    );
    assert_eq!(
        redis.exec(cmd::GetDel(&key)).await.unwrap().unwrap(),
        "Hello Redis"
    );
    assert_eq!(redis.exec(cmd::GetDel(&key)).await.unwrap(), None);
    assert_eq!(redis.exec(cmd::StrLen(&key)).await.unwrap(), 0);

    // getex
    assert_eq!(redis.exec(cmd::GetEx(&key)).await.unwrap(), None);
    redis.exec(cmd::Set(&key, "v")).await.unwrap();
    assert_eq!(redis.exec(cmd::GetEx(&key)).await.unwrap().unwrap(), "v");
    redis
        .exec(cmd::GetEx(&key).expire_millis(10_000))
        .await
        .unwrap();
    assert!(matches!(
        redis.exec(cmd::PTtl(&key)).await.unwrap(),
        cmd::TtlResult::Millis(1..=10_000)
    ));
    redis.exec(cmd::GetEx(&key).persist()).await.unwrap();
    assert_eq!(
        redis.exec(cmd::Ttl(&key)).await.unwrap(),
        cmd::TtlResult::NoExpire
    );
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    redis
        .exec(cmd::GetEx(&key).expire_at_secs(now.as_secs() as i64 + 100))
        .await
        .unwrap();
    assert!(matches!(
        redis.exec(cmd::Ttl(&key)).await.unwrap(),
        cmd::TtlResult::Seconds(1..=100)
    ));
    redis
        .exec(cmd::GetEx(&key).expire_at_millis(now.as_millis() as i64 + 100_000))
        .await
        .unwrap();
    assert!(matches!(
        redis.exec(cmd::Ttl(&key)).await.unwrap(),
        cmd::TtlResult::Seconds(1..=100)
    ));

    // set with absolute expiration
    redis
        .exec(cmd::Set(&key, "v").expire_at_secs(now.as_secs() as i64 + 100))
        .await
        .unwrap();
    assert!(matches!(
        redis.exec(cmd::Ttl(&key)).await.unwrap(),
        cmd::TtlResult::Seconds(1..=100)
    ));
    redis
        .exec(cmd::Set(&key, "v").expire_at_millis(now.as_millis() as i64 + 100_000))
        .await
        .unwrap();
    assert!(matches!(
        redis.exec(cmd::Ttl(&key)).await.unwrap(),
        cmd::TtlResult::Seconds(1..=100)
    ));
    redis.exec(cmd::Del(&key)).await.unwrap();

    // mset / mget
    let key2 = new_key();
    let mut mset = cmd::MSet(&key, "1");
    mset.add_entry(&key2, "2");
    assert!(redis.exec(mset).await.unwrap());
    assert_eq!(
        redis
            .exec(cmd::MGet(&key).keys([&key2, &new_key()]))
            .await
            .unwrap(),
        vec![Some(Bytes::from("1")), Some(Bytes::from("2")), None]
    );
    let key3 = new_key();
    assert!(
        !redis
            .exec(cmd::MSet(&key, "x").entry(&key3, "y").if_not_exists())
            .await
            .unwrap()
    );
    assert_eq!(redis.exec(cmd::Get(&key3)).await.unwrap(), None);
    assert!(
        redis
            .exec(cmd::MSet(&key3, "y").if_not_exists())
            .await
            .unwrap()
    );
    redis
        .exec(cmd::Del(&key).key(&key2).key(&key3))
        .await
        .unwrap();
}

#[ntex::test]
async fn test_keys_commands() {
    let redis = connect().await;
    let key = new_key();
    let key2 = new_key();

    redis.exec(cmd::Set(&key, "v")).await.unwrap();
    assert_eq!(redis.exec(cmd::Type(&key)).await.unwrap(), "string");
    assert_eq!(redis.exec(cmd::Type(&key2)).await.unwrap(), "none");
    assert_eq!(redis.exec(cmd::Touch(&key).key(&key2)).await.unwrap(), 1);

    // expiration
    assert_eq!(
        redis.exec(cmd::PTtl(&key)).await.unwrap(),
        cmd::TtlResult::NoExpire
    );
    assert_eq!(
        redis.exec(cmd::PTtl(&key2)).await.unwrap(),
        cmd::TtlResult::NotFound
    );
    assert!(!redis.exec(cmd::Persist(&key)).await.unwrap());
    assert!(redis.exec(cmd::PExpire(&key, 10_000)).await.unwrap());
    assert!(matches!(
        redis.exec(cmd::PTtl(&key)).await.unwrap(),
        cmd::TtlResult::Millis(1..=10_000)
    ));
    assert!(redis.exec(cmd::Persist(&key)).await.unwrap());
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    assert!(
        redis
            .exec(cmd::PExpireAt(&key, now + 100_000))
            .await
            .unwrap()
    );
    assert!(matches!(
        redis.exec(cmd::Ttl(&key)).await.unwrap(),
        cmd::TtlResult::Seconds(1..=100)
    ));
    assert!(!redis.exec(cmd::PExpire(&key2, 1000)).await.unwrap());

    // rename
    redis.exec(cmd::Rename(&key, &key2)).await.unwrap();
    assert_eq!(redis.exec(cmd::Exists(&key)).await.unwrap(), 0);
    assert!(redis.exec(cmd::Rename(&key, &key2)).await.is_err());
    redis.exec(cmd::Set(&key, "v2")).await.unwrap();
    assert!(!redis.exec(cmd::RenameNx(&key, &key2)).await.unwrap());
    let key3 = new_key();
    assert!(redis.exec(cmd::RenameNx(&key, &key3)).await.unwrap());

    assert_eq!(
        redis
            .exec(cmd::Unlink(&key).keys([&key2, &key3]))
            .await
            .unwrap(),
        2
    );
}

#[ntex::test]
async fn test_scan() {
    let redis = connect().await;
    let prefix = new_key();
    let keys: Vec<_> = (0..25).map(|i| format!("{prefix}:{i}")).collect();
    for key in &keys {
        redis.exec(cmd::Set(key, "v")).await.unwrap();
    }
    let hkey = format!("{prefix}:hash");
    redis.exec(cmd::HSet(&hkey, "f", "v")).await.unwrap();

    let mut found = Vec::new();
    let mut cursor = 0;
    loop {
        let (next, batch) = redis
            .exec(
                cmd::Scan(cursor)
                    .pattern(format!("{prefix}:*"))
                    .count(10)
                    .key_type("string"),
            )
            .await
            .unwrap();
        found.extend(batch);
        if next == 0 {
            break;
        }
        cursor = next;
    }
    found.sort();
    found.dedup();
    let mut expected: Vec<_> = keys.iter().map(|k| Bytes::from(k.clone())).collect();
    expected.sort();
    assert_eq!(found, expected);

    // hscan
    let (cursor, items) = redis.exec(cmd::HScan(&hkey, 0)).await.unwrap();
    assert_eq!(cursor, 0);
    assert_eq!(items, vec![(Bytes::from("f"), Bytes::from("v"))]);

    // sscan
    let skey = new_key();
    redis
        .exec(cmd::SAdd(&skey, "a").members(["b", "c"]))
        .await
        .unwrap();
    let (cursor, mut items) = redis
        .exec(cmd::SScan(&skey, 0).pattern("[ab]"))
        .await
        .unwrap();
    items.sort();
    assert_eq!(cursor, 0);
    assert_eq!(items, ["a", "b"]);

    // zscan
    let zkey = new_key();
    redis
        .exec(cmd::ZAdd(&zkey, 1.5, "a").entry(f64::INFINITY, "b"))
        .await
        .unwrap();
    let (cursor, mut items) = redis.exec(cmd::ZScan(&zkey, 0).count(100)).await.unwrap();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(cursor, 0);
    assert_eq!(
        items,
        vec![(Bytes::from("a"), 1.5), (Bytes::from("b"), f64::INFINITY)]
    );

    redis
        .exec(cmd::Del(&hkey).keys(&keys).key(&skey).key(&zkey))
        .await
        .unwrap();
}

#[ntex::test]
async fn test_lists_commands() {
    let redis = connect().await;
    let key = new_key();
    let key2 = new_key();

    assert_eq!(redis.exec(cmd::LLen(&key)).await.unwrap(), 0);
    assert!(
        redis
            .exec(cmd::LRange(&key, 0, -1))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(redis.exec(cmd::LInsert(&key, "a", "b")).await.unwrap(), 0);

    redis
        .exec(cmd::RPush(&key, "a").extend(["b", "a", "c", "a"]))
        .await
        .unwrap();
    assert_eq!(redis.exec(cmd::LLen(&key)).await.unwrap(), 5);
    assert_eq!(
        redis.exec(cmd::LRange(&key, 1, 2)).await.unwrap(),
        ["b", "a"]
    );

    // lpos
    assert_eq!(redis.exec(cmd::LPos(&key, "a")).await.unwrap(), Some(0));
    assert_eq!(
        redis.exec(cmd::LPos(&key, "a").rank(-1)).await.unwrap(),
        Some(4)
    );
    assert_eq!(
        redis.exec(cmd::LPos(&key, "c").max_len(2)).await.unwrap(),
        None
    );

    // lrem, lset, linsert, ltrim
    assert_eq!(redis.exec(cmd::LRem(&key, 2, "a")).await.unwrap(), 2);
    assert_eq!(
        redis.exec(cmd::LRange(&key, 0, -1)).await.unwrap(),
        ["b", "c", "a"]
    );
    redis.exec(cmd::LSet(&key, 0, "x")).await.unwrap();
    assert!(redis.exec(cmd::LSet(&key, 10, "x")).await.is_err());
    assert_eq!(redis.exec(cmd::LInsert(&key, "c", "y")).await.unwrap(), 4);
    assert_eq!(
        redis
            .exec(cmd::LInsert(&key, "c", "z").after())
            .await
            .unwrap(),
        5
    );
    assert_eq!(redis.exec(cmd::LInsert(&key, "q", "z")).await.unwrap(), -1);
    assert_eq!(
        redis.exec(cmd::LRange(&key, 0, -1)).await.unwrap(),
        ["x", "y", "c", "z", "a"]
    );
    redis.exec(cmd::LTrim(&key, 1, -2)).await.unwrap();
    assert_eq!(
        redis.exec(cmd::LRange(&key, 0, -1)).await.unwrap(),
        ["y", "c", "z"]
    );

    // lmove
    assert_eq!(
        redis
            .exec(cmd::LMove(&key, &key2, cmd::Side::Left, cmd::Side::Right))
            .await
            .unwrap()
            .unwrap(),
        "y"
    );
    assert_eq!(
        redis
            .exec(cmd::LMove(&key, &key2, cmd::Side::Right, cmd::Side::Left))
            .await
            .unwrap()
            .unwrap(),
        "z"
    );
    assert_eq!(
        redis.exec(cmd::LRange(&key2, 0, -1)).await.unwrap(),
        ["z", "y"]
    );
    assert_eq!(
        redis
            .exec(cmd::LMove(
                new_key(),
                &key2,
                cmd::Side::Left,
                cmd::Side::Left
            ))
            .await
            .unwrap(),
        None
    );

    redis.exec(cmd::Del(&key).key(&key2)).await.unwrap();
}

#[ntex::test]
async fn test_hashes_commands() {
    let redis = connect().await;
    let key = new_key();

    assert!(redis.exec(cmd::HSetNx(&key, "a", "1")).await.unwrap());
    assert!(!redis.exec(cmd::HSetNx(&key, "a", "2")).await.unwrap());
    redis.exec(cmd::HSet(&key, "b", "hello")).await.unwrap();

    assert!(redis.exec(cmd::HExists(&key, "a")).await.unwrap());
    assert!(!redis.exec(cmd::HExists(&key, "c")).await.unwrap());
    assert_eq!(redis.exec(cmd::HStrLen(&key, "b")).await.unwrap(), 5);
    assert_eq!(redis.exec(cmd::HStrLen(&key, "c")).await.unwrap(), 0);

    let mut keys = redis.exec(cmd::HKeys(&key)).await.unwrap();
    keys.sort();
    assert_eq!(keys, ["a", "b"]);
    let mut vals = redis.exec(cmd::HVals(&key)).await.unwrap();
    vals.sort();
    assert_eq!(vals, ["1", "hello"]);

    assert_eq!(
        redis
            .exec(cmd::HMGet(&key, "b").fields(["c", "a"]))
            .await
            .unwrap(),
        vec![Some(Bytes::from("hello")), None, Some(Bytes::from("1"))]
    );

    assert_eq!(
        redis.exec(cmd::HIncrByFloat(&key, "a", 0.5)).await.unwrap(),
        1.5
    );
    assert_eq!(
        redis
            .exec(cmd::HIncrByFloat(&key, "n", -2.25))
            .await
            .unwrap(),
        -2.25
    );
    assert!(redis.exec(cmd::HIncrByFloat(&key, "b", 1.0)).await.is_err());

    redis.exec(cmd::Del(&key)).await.unwrap();
    assert!(redis.exec(cmd::HKeys(&key)).await.unwrap().is_empty());
}

#[ntex::test]
async fn test_sets() {
    let redis = connect().await;
    let key = new_key();
    let key2 = new_key();

    assert_eq!(
        redis
            .exec(cmd::SAdd(&key, "a").members(["b", "c", "a"]))
            .await
            .unwrap(),
        3
    );
    assert_eq!(redis.exec(cmd::SAdd(&key, "a")).await.unwrap(), 0);
    assert_eq!(redis.exec(cmd::SCard(&key)).await.unwrap(), 3);
    assert!(redis.exec(cmd::SIsMember(&key, "a")).await.unwrap());
    assert!(!redis.exec(cmd::SIsMember(&key, "x")).await.unwrap());
    assert_eq!(
        redis
            .exec(cmd::SMIsMember(&key, "a").member("x").members(["c"]))
            .await
            .unwrap(),
        vec![true, false, true]
    );

    redis.exec(cmd::SAdd(&key2, "c").member("d")).await.unwrap();
    let mut res = redis.exec(cmd::SUnion(&key).key(&key2)).await.unwrap();
    res.sort();
    assert_eq!(res, ["a", "b", "c", "d"]);
    assert_eq!(
        redis.exec(cmd::SInter(&key).keys([&key2])).await.unwrap(),
        ["c"]
    );
    let mut res = redis.exec(cmd::SDiff(&key).key(&key2)).await.unwrap();
    res.sort();
    assert_eq!(res, ["a", "b"]);

    assert_eq!(
        redis
            .exec(cmd::SRem(&key, "a").members(["b", "x"]))
            .await
            .unwrap(),
        2
    );
    assert_eq!(redis.exec(cmd::SMembers(&key)).await.unwrap(), ["c"]);
    assert_eq!(
        redis.exec(cmd::SRandMember(&key)).await.unwrap().unwrap(),
        "c"
    );
    assert_eq!(redis.exec(cmd::SPop(&key)).await.unwrap().unwrap(), "c");
    assert_eq!(redis.exec(cmd::SPop(&key)).await.unwrap(), None);
    assert_eq!(redis.exec(cmd::SRandMember(&key)).await.unwrap(), None);
    assert!(redis.exec(cmd::SMembers(&key)).await.unwrap().is_empty());

    redis.exec(cmd::Del(&key2)).await.unwrap();
}

#[ntex::test]
async fn test_sorted_sets() {
    let redis = connect().await;
    let key = new_key();

    let mut zadd = cmd::ZAdd(&key, 1.0, "a").entry(2.0, "b");
    zadd.add_entry(3.0, "c");
    assert_eq!(redis.exec(zadd).await.unwrap(), 3);
    assert_eq!(redis.exec(cmd::ZCard(&key)).await.unwrap(), 3);

    // flags
    assert_eq!(
        redis
            .exec(cmd::ZAdd(&key, 10.0, "a").entry(1.0, "d").if_exists().ch())
            .await
            .unwrap(),
        1
    );
    assert_eq!(redis.exec(cmd::ZScore(&key, "d")).await.unwrap(), None);
    assert_eq!(
        redis
            .exec(cmd::ZAdd(&key, 0.0, "a").entry(4.0, "d").if_not_exists())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        redis.exec(cmd::ZScore(&key, "a")).await.unwrap(),
        Some(10.0)
    );
    assert_eq!(
        redis
            .exec(cmd::ZAdd(&key, 5.0, "a").gt().ch())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        redis
            .exec(cmd::ZAdd(&key, 5.0, "a").lt().ch())
            .await
            .unwrap(),
        1
    );

    // a=5 b=2 c=3 d=4
    assert_eq!(
        redis.exec(cmd::ZIncrBy(&key, -4.5, "a")).await.unwrap(),
        0.5
    );
    assert_eq!(redis.exec(cmd::ZCount(&key, 2, 3)).await.unwrap(), 2);
    assert_eq!(
        redis.exec(cmd::ZCount(&key, "(2", "+inf")).await.unwrap(),
        2
    );
    assert_eq!(redis.exec(cmd::ZRank(&key, "a")).await.unwrap(), Some(0));
    assert_eq!(redis.exec(cmd::ZRevRank(&key, "a")).await.unwrap(), Some(3));
    assert_eq!(redis.exec(cmd::ZRank(&key, "x")).await.unwrap(), None);

    // ranges
    assert_eq!(
        redis.exec(cmd::ZRange(&key, 0, -1)).await.unwrap(),
        ["a", "b", "c", "d"]
    );
    assert_eq!(
        redis.exec(cmd::ZRange(&key, 0, 1).rev()).await.unwrap(),
        ["d", "c"]
    );
    assert_eq!(
        redis
            .exec(cmd::ZRange(&key, "+inf", 2.0).by_score().rev().limit(1, 2))
            .await
            .unwrap(),
        ["c", "b"]
    );
    assert_eq!(
        redis
            .exec(cmd::ZRange(&key, 0, 1).with_scores())
            .await
            .unwrap(),
        vec![(Bytes::from("a"), 0.5), (Bytes::from("b"), 2.0)]
    );
    assert!(
        redis
            .exec(cmd::ZRange(&key, 0, 1).limit(0, 1))
            .await
            .is_err()
    );

    // lex range
    let lkey = new_key();
    redis
        .exec(cmd::ZAdd(&lkey, 0.0, "a").entry(0.0, "b").entry(0.0, "c"))
        .await
        .unwrap();
    assert_eq!(
        redis
            .exec(cmd::ZRange(&lkey, "(a", "+").by_lex())
            .await
            .unwrap(),
        ["b", "c"]
    );

    // pop
    assert_eq!(
        redis.exec(cmd::ZPopMin(&key)).await.unwrap(),
        vec![(Bytes::from("a"), 0.5)]
    );
    assert_eq!(
        redis.exec(cmd::ZPopMax(&key).count(2)).await.unwrap(),
        vec![(Bytes::from("d"), 4.0), (Bytes::from("c"), 3.0)]
    );
    assert_eq!(
        redis
            .exec(cmd::ZRem(&key, "b").members(["x"]))
            .await
            .unwrap(),
        1
    );
    assert!(redis.exec(cmd::ZPopMin(&key)).await.unwrap().is_empty());

    redis.exec(cmd::Del(&lkey)).await.unwrap();
}

#[ntex::test]
async fn test_server_commands() {
    let redis = connect().await;
    assert!(redis.exec(cmd::Select(14)).await.unwrap());

    let key = new_key();
    redis.exec(cmd::Set(&key, "v")).await.unwrap();
    assert!(redis.exec(cmd::DbSize()).await.unwrap() >= 1);
    redis.exec(cmd::FlushDb()).await.unwrap();
    assert_eq!(redis.exec(cmd::DbSize()).await.unwrap(), 0);
    redis.exec(cmd::Set(&key, "v")).await.unwrap();
    redis.exec(cmd::FlushDb().asynchronous()).await.unwrap();
    assert_eq!(redis.exec(cmd::Get(&key)).await.unwrap(), None);

    let info = redis.exec(cmd::Info()).await.unwrap();
    assert!(info.contains("redis_version:"));
    let info = redis
        .exec(cmd::Info().section("server").section("memory"))
        .await
        .unwrap();
    assert!(info.contains("used_memory:"));
    assert!(!info.contains("connected_clients:"));
}

#[ntex::test]
async fn test_connection_commands() {
    let redis = connect().await;

    assert_eq!(redis.exec(cmd::Echo("hello")).await.unwrap(), "hello");
    assert_eq!(redis.exec(cmd::ClientGetName()).await.unwrap(), None);
    redis.exec(cmd::ClientSetName("ntex-redis")).await.unwrap();
    assert_eq!(
        redis.exec(cmd::ClientGetName()).await.unwrap().unwrap(),
        "ntex-redis"
    );
    assert!(redis.exec(cmd::ClientSetName("bad name")).await.is_err());
}
