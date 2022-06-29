use ntex_redis::{cmd, RedisConnector};
use std::error::Error;
use std::rc::Rc;

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // subscriber
    let client = Rc::new(
        RedisConnector::new("127.0.0.1:6379")
            .connect_simple()
            .await?,
    );

    let client_clone = client.clone();

    ntex::rt::spawn(async move {
        let subscriber = client_clone.stream(cmd::Subscribe("pubsub")).unwrap();

        loop {
            match subscriber.recv().await {
                Some(Ok(cmd::SubscribeItem::Subscribed(channel))) => {
                    println!("sub: subscribed to {:?}", channel)
                }
                Some(Ok(cmd::SubscribeItem::Message { channel, payload })) => {
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
        redis.exec(cmd::Publish("pubsub", &value)).await?;
    }

    // unsubscribe
    client.send(cmd::UnSubscribe("pubsub"))?;

    // allow to subscriber recv unsubscribe message
    ntex::time::sleep(ntex::time::Millis(10)).await;

    Ok(())
}
