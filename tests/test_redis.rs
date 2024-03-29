use ntex::util::{Bytes, HashMap};
use ntex_redis::{cmd, Client, RedisConnector};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::time::{Duration, SystemTime};

async fn connect() -> Client {
    RedisConnector::new("127.0.0.1:6379")
        .connect()
        .await
        .unwrap()
}

fn new_key() -> String {
    thread_rng()
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
    assert!(result.is_err());
}

#[ntex::test]
async fn test_auth_simple() {
    let result = RedisConnector::new("127.0.0.1:6379")
        .password("test")
        .connect_simple()
        .await;
    assert!(result.is_err());
}

#[ntex::test]
async fn test_strings() {
    env_logger::init();
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
    assert_eq!(resp, cmd::TtlResult::Seconds(10));
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
