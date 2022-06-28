use futures_util::StreamExt;
use ntex_redis::{cmd, RedisConnector};
use std::error::Error;

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // subscribe
    let client = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await?;
    let mut subscriber = client.stream(cmd::Subscribe("pubsub"))?;

    ntex::rt::spawn(async move {
        loop {
            match subscriber.next().await {
                Some(Ok(cmd::SubscribeItem::Subscribed)) => println!("sub: subscribed"),
                Some(Ok(cmd::SubscribeItem::Message(payload))) => {
                    println!("sub: {:?}", payload)
                }
                Some(Err(e)) => {
                    println!("sub: {}", e);
                    return;
                }
                _ => unreachable!(),
            }
        }
    });

    // publish
    let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;

    for i in 0..5 {
        let value = i.to_string();
        println!("pub: {}", value);
        redis.exec(cmd::Publish("pubsub", &value)).await?;
    }

    Ok(())
}
