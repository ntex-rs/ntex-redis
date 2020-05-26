use ntex_redis::{cmd, RedisConnector};
use std::error::Error;

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;

    redis.exec(cmd::Set("test", "value")).await?;

    if let Some(resp) = redis.exec(cmd::Get("test")).await? {
        assert_eq!(resp, "value");
    }

    Ok(())
}
