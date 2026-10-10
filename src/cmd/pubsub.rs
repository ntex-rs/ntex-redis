use super::{Command, CommandError, utils};
use ntex_bytes::Bytes;
use ntex_util::future::Either;
use std::convert::TryFrom;

use crate::codec::{BulkString, Request, Response};

const TYPE_SUBSCRIBE: Bytes = Bytes::from_static(b"subscribe");
const TYPE_UNSUBSCRIBE: Bytes = Bytes::from_static(b"unsubscribe");
const TYPE_SSUBSCRIBE: Bytes = Bytes::from_static(b"ssubscribe");
const TYPE_SUNSUBSCRIBE: Bytes = Bytes::from_static(b"sunsubscribe");
const TYPE_PSUBSCRIBE: Bytes = Bytes::from_static(b"psubscribe");
const TYPE_PUNSUBSCRIBE: Bytes = Bytes::from_static(b"punsubscribe");
const TYPE_MESSAGE: Bytes = Bytes::from_static(b"message");
const TYPE_SMESSAGE: Bytes = Bytes::from_static(b"smessage");
const TYPE_PMESSAGE: Bytes = Bytes::from_static(b"pmessage");

/// Marker trait for commands that can be sent over a subscribed connection,
/// see [`SubscriptionClient::send`](crate::SubscriptionClient::send)
pub trait PubSubCommand {}

/// Item received from a subscribed connection
#[derive(Debug, PartialEq, Eq)]
pub enum SubscribeItem {
    /// Subscribed to a channel or pattern
    Subscribed(Bytes),
    /// Unsubscribed from a channel or pattern
    UnSubscribed(Bytes),
    /// Published message
    Message {
        /// Matched pattern, set for messages received via `PSUBSCRIBE`
        pattern: Option<Bytes>,
        /// Channel name
        channel: Bytes,
        /// Message payload
        payload: Bytes,
    },
}

struct MessagePayload(Either<Bytes, i64>);

impl TryFrom<Response> for MessagePayload {
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<Self, Self::Error> {
        match val {
            Response::Bytes(bytes) => Ok(MessagePayload(Either::Left(bytes))),
            Response::Integer(number) => Ok(MessagePayload(Either::Right(number))),
            _ => Err(("Not a bytes object or integer", val)),
        }
    }
}

impl TryFrom<Response> for SubscribeItem {
    type Error = CommandError;

    fn try_from(val: Response) -> Result<Self, Self::Error> {
        let (mtype, pattern, channel, payload) = match val {
            Response::Array(ary) => match ary.len() {
                // subscribe or ssubscribe message
                3 => {
                    let mut ary_iter = ary.into_iter();
                    (
                        Bytes::try_from(ary_iter.next().expect("No value"))?,
                        None,
                        Bytes::try_from(ary_iter.next().expect("No value"))?,
                        MessagePayload::try_from(ary_iter.next().expect("No value"))?,
                    )
                }
                // psubscribe message
                4 => {
                    let mut ary_iter = ary.into_iter();
                    (
                        Bytes::try_from(ary_iter.next().expect("No value"))?,
                        Some(Bytes::try_from(ary_iter.next().expect("No value"))?),
                        Bytes::try_from(ary_iter.next().expect("No value"))?,
                        MessagePayload::try_from(ary_iter.next().expect("No value"))?,
                    )
                }
                _ => {
                    return Err(CommandError::Output(
                        "Array needs to be 3 or 4 elements",
                        Response::Array(ary),
                    ));
                }
            },
            _ => return Err(CommandError::Output("Unexpected value", val)),
        };

        match &mtype {
            s if s == &TYPE_SUBSCRIBE || s == &TYPE_SSUBSCRIBE || s == &TYPE_PSUBSCRIBE => {
                Ok(SubscribeItem::Subscribed(channel))
            }
            s if s == &TYPE_UNSUBSCRIBE || s == &TYPE_SUNSUBSCRIBE || s == &TYPE_PUNSUBSCRIBE => {
                Ok(SubscribeItem::UnSubscribed(channel))
            }
            s if s == &TYPE_MESSAGE || s == &TYPE_SMESSAGE || s == &TYPE_PMESSAGE => {
                if let Some(payload) = payload.0.left() {
                    Ok(SubscribeItem::Message {
                        pattern,
                        channel,
                        payload,
                    })
                } else {
                    Err(CommandError::Output(
                        "Subscription message payload is not bytes",
                        Response::Nil,
                    ))
                }
            }
            _ => Err(CommandError::Output(
                "Subscription message type unknown",
                Response::Bytes(mtype),
            )),
        }
    }
}

/// SUBSCRIBE, SSUBSCRIBE and PSUBSCRIBE command, see [`Subscribe`]
pub struct SubscribeOutputCommand(pub(crate) Request);

impl Command for SubscribeOutputCommand {
    type Output = SubscribeItem;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        SubscribeItem::try_from(val)
    }
}

/// UNSUBSCRIBE, SUNSUBSCRIBE and PUNSUBSCRIBE command, see [`UnSubscribe`]
pub struct UnSubscribeOutputCommand(pub(crate) Request);

impl Command for UnSubscribeOutputCommand {
    type Output = SubscribeItem;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        SubscribeItem::try_from(val)
    }
}

/// PUBLISH redis command
///
/// Post a message to the given channel. Returns the number of clients
/// that received the message.
pub fn Publish<T, V>(key: T, value: V) -> utils::IntOutputCommand
where
    BulkString: From<T> + From<V>,
{
    utils::IntOutputCommand(Request::Array(vec![
        Request::from_static("PUBLISH"),
        Request::BulkString(key.into()),
        Request::BulkString(value.into()),
    ]))
}

/// SPUBLISH redis command
///
/// Post a message to the given shard channel. Returns the number of clients
/// that received the message.
pub fn SPublish<T, V>(key: T, value: V) -> utils::IntOutputCommand
where
    BulkString: From<T> + From<V>,
{
    utils::IntOutputCommand(Request::Array(vec![
        Request::from_static("SPUBLISH"),
        Request::BulkString(key.into()),
        Request::BulkString(value.into()),
    ]))
}

/// SUBSCRIBE redis command
///
/// Subscribe to the given channels. Use with [`SimpleClient::subscribe`](crate::SimpleClient::subscribe),
/// the connection can only be used for pub/sub commands after this.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let subscriber = RedisConnector::new("127.0.0.1:6379")
///         .connect_simple()
///         .await?
///         .subscribe(cmd::Subscribe(vec!["my-channel"]))?;
///
///     // first item is subscription confirmation
///     let item = subscriber.recv().await.unwrap()?;
///     assert_eq!(item, cmd::SubscribeItem::Subscribed("my-channel".into()));
///
///     redis.exec(cmd::Publish("my-channel", "hello")).await?;
///
///     let item = subscriber.recv().await.unwrap()?;
///     assert!(matches!(item, cmd::SubscribeItem::Message { payload, .. } if payload == "hello"));
///     Ok(())
/// }
/// ```
pub fn Subscribe<T>(channels: Vec<T>) -> SubscribeOutputCommand
where
    BulkString: From<T>,
{
    let mut req = Request::from_static("SUBSCRIBE");
    for channel in channels {
        req = req.add(Request::BulkString(channel.into()));
    }
    SubscribeOutputCommand(req)
}

/// UNSUBSCRIBE redis command
///
/// Unsubscribe from the given channels, or from all channels if `None`.
/// Use with [`SubscriptionClient::send`](crate::SubscriptionClient::send).
pub fn UnSubscribe<T>(channels: Option<Vec<T>>) -> UnSubscribeOutputCommand
where
    BulkString: From<T>,
{
    let mut req = Request::from_static("UNSUBSCRIBE");
    if let Some(channels) = channels {
        for channel in channels {
            req = req.add(Request::BulkString(channel.into()));
        }
    }
    UnSubscribeOutputCommand(req)
}

/// SSUBSCRIBE redis command
///
/// Subscribe to the given shard channels, see [`Subscribe`].
pub fn SSubscribe<T>(channels: Vec<T>) -> SubscribeOutputCommand
where
    BulkString: From<T>,
{
    let mut req = Request::from_static("SSUBSCRIBE");
    for channel in channels {
        req = req.add(Request::BulkString(channel.into()));
    }
    SubscribeOutputCommand(req)
}

/// SUNSUBSCRIBE redis command
///
/// Unsubscribe from the given shard channels, or from all shard channels if `None`.
pub fn SUnSubscribe<T>(channels: Option<Vec<T>>) -> UnSubscribeOutputCommand
where
    BulkString: From<T>,
{
    let mut req = Request::from_static("SUNSUBSCRIBE");
    if let Some(channels) = channels {
        for channel in channels {
            req = req.add(Request::BulkString(channel.into()));
        }
    }
    UnSubscribeOutputCommand(req)
}

/// PSUBSCRIBE redis command
///
/// Subscribe to channels matching the given glob-style patterns, see [`Subscribe`].
pub fn PSubscribe<T>(channels: Vec<T>) -> SubscribeOutputCommand
where
    BulkString: From<T>,
{
    let mut req = Request::from_static("PSUBSCRIBE");
    for channel in channels {
        req = req.add(Request::BulkString(channel.into()));
    }
    SubscribeOutputCommand(req)
}

/// PUNSUBSCRIBE redis command
///
/// Unsubscribe from the given patterns, or from all patterns if `None`.
pub fn PUnSubscribe<T>(channels: Option<Vec<T>>) -> UnSubscribeOutputCommand
where
    BulkString: From<T>,
{
    let mut req = Request::from_static("PUNSUBSCRIBE");
    if let Some(channels) = channels {
        for channel in channels {
            req = req.add(Request::BulkString(channel.into()));
        }
    }
    UnSubscribeOutputCommand(req)
}

impl PubSubCommand for SubscribeOutputCommand {}
impl PubSubCommand for UnSubscribeOutputCommand {}
