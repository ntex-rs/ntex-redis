use super::{utils, Command, CommandError};
use ntex::util::{Bytes, Either};
use std::convert::TryFrom;

use crate::codec::{BulkString, Request, Response};

const TYPE_SUBSCRIBE: Bytes = Bytes::from_static(b"subscribe");
const TYPE_UNSUBSCRIBE: Bytes = Bytes::from_static(b"unsubscribe");
const TYPE_MESSAGE: Bytes = Bytes::from_static(b"message");

pub trait PubSubCommand {}

#[derive(Debug, PartialEq)]
pub enum SubscribeItem {
    Subscribed(Bytes),
    UnSubscribed(Bytes),
    Message { channel: Bytes, payload: Bytes },
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
        let (mtype, channel, payload) = <(Bytes, Bytes, MessagePayload)>::try_from(val)?;

        if mtype == &TYPE_SUBSCRIBE {
            return Ok(SubscribeItem::Subscribed(channel));
        }

        if mtype == &TYPE_UNSUBSCRIBE {
            return Ok(SubscribeItem::UnSubscribed(channel));
        }

        if mtype != &TYPE_MESSAGE {
            return Err(CommandError::Output(
                "Subscription message type unknown",
                Response::Bytes(mtype),
            ));
        }

        let payload = if let Some(payload) = payload.0.left() {
            payload
        } else {
            return Err(CommandError::Output(
                "Subscription message payload is not bytes",
                Response::Nil,
            ));
        };

        Ok(SubscribeItem::Message { channel, payload })
    }
}

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

/// Publish redis command
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

/// Subscribe redis command
pub fn Subscribe<T>(key: T) -> SubscribeOutputCommand
where
    BulkString: From<T>,
{
    SubscribeOutputCommand(Request::Array(vec![
        Request::from_static("SUBSCRIBE"),
        Request::BulkString(key.into()),
    ]))
}

/// Unsubscribe redis command
pub fn UnSubscribe<T>(key: T) -> UnSubscribeOutputCommand
where
    BulkString: From<T>,
{
    UnSubscribeOutputCommand(Request::Array(vec![
        Request::from_static("UNSUBSCRIBE"),
        Request::BulkString(key.into()),
    ]))
}

impl PubSubCommand for SubscribeOutputCommand {}
impl PubSubCommand for UnSubscribeOutputCommand {}
