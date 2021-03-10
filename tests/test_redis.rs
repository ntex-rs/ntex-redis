use ntex_redis::{cmd, Client, RedisConnector};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

async fn connect() -> Client {
    RedisConnector::new("127.0.0.1:6379")
        .connect()
        .await
        .unwrap()
}

fn new_key() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(12).collect()
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
        + Duration::from_secs(10);
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
    let mut redis = RedisConnector::new("127.0.0.1:6379")
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
        .exec(cmd::HSet(&key, "field1", "1").insert("field2", "2"))
        .await
        .unwrap();
    assert_eq!(result, 2);

    let result = redis.exec(cmd::HIncrBy(&key, "field1", 10)).await.unwrap();
    assert_eq!(result, 11);

    let result = redis.exec(cmd::HGet(&key, "field1")).await.unwrap();
    assert_eq!(result.unwrap(), "11");

    let result = redis.exec(cmd::HGetAll(&key)).await.unwrap();
    let mut expected = HashMap::new();
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
