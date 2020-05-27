use ntex_redis::{cmd, Client, RedisConnector};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

async fn connect() -> (String, Client) {
    let redis = RedisConnector::new("127.0.0.1:6379")
        .connect()
        .await
        .unwrap();
    let key: String = thread_rng().sample_iter(&Alphanumeric).take(12).collect();

    (key, redis)
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
    let (key, redis) = connect().await;
    let result = redis.exec(cmd::Set(&key, "value")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Get(&key)).await.unwrap().unwrap();
    assert_eq!(resp, "value");

    let resp = redis.exec(cmd::Get("unknown")).await.unwrap();
    assert_eq!(resp, None);
}

#[ntex::test]
async fn test_keys() {
    let (key1, redis) = connect().await;
    let key2: String = thread_rng().sample_iter(&Alphanumeric).take(12).collect();

    let result = redis.exec(cmd::Set(&key1, "value")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Exists(&key1)).await.unwrap();
    assert_eq!(resp, 1);
    let resp = redis.exec(cmd::Exists(&key2)).await.unwrap();
    assert_eq!(resp, 0);
    let resp = redis.exec(cmd::Exists(&key1).key(&key2)).await.unwrap();
    assert_eq!(resp, 1);
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
    let key: String = thread_rng().sample_iter(&Alphanumeric).take(12).collect();

    let result = redis.exec(cmd::Set(&key, "value")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Get(&key)).await.unwrap().unwrap();
    assert_eq!(resp, "value");

    let resp = redis.exec(cmd::Get("unknown")).await.unwrap();
    assert_eq!(resp, None);
}

#[ntex::test]
async fn test_lists() {
    let (key, redis) = connect().await;

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
