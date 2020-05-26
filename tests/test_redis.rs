use ntex_redis::{cmd, RedisConnector};

#[ntex::test]
async fn teste_auth() {
    let result = RedisConnector::new("127.0.0.1:6379")
        .password("test")
        .connect()
        .await;
    assert!(result.is_err());
}

#[ntex::test]
async fn teste_get_set() {
    env_logger::init();

    let redis = RedisConnector::new("127.0.0.1:6379")
        .connect()
        .await
        .unwrap();
    let result = redis.exec(cmd::Set("test", "value")).await.unwrap();
    assert!(result);

    let resp = redis.exec(cmd::Get("test")).await.unwrap().unwrap();
    assert_eq!(resp, "value");

    let resp = redis.exec(cmd::Get("unknown")).await.unwrap();
    assert_eq!(resp, None);
}
