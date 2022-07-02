use ntex_redis::{cmd, RedisConnector};
use std::error::Error;

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let channel = "pubsub_channel";

    // subscriber
    let client = RedisConnector::new("127.0.0.1:6379")
        .connect_simple()
        .await?;

    let pubsub = client.subscribe(cmd::Subscribe(vec![channel])).unwrap();

    ntex::rt::spawn(async move {
        loop {
            match pubsub.recv().await {
                Some(Ok(cmd::SubscribeItem::Subscribed(channel))) => {
                    println!("sub: subscribed to {:?}", channel)
                }
                Some(Ok(cmd::SubscribeItem::Message {
                    pattern: _,
                    channel,
                    payload,
                })) => {
                    println!("sub: {:?} from {:?}", payload, channel)
                }
                Some(Ok(cmd::SubscribeItem::UnSubscribed(channel))) => {
                    println!("sub: unsubscribed from {:?}", channel)
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
        redis.exec(cmd::Publish(channel, &value)).await?;
    }

    Ok(())
}
