use super::{utils, Command, CommandError};
use ntex::util::Bytes;
use std::convert::TryFrom;

use crate::codec::{BulkString, Request, Response};

const TYPE_SUBSCRIBE: Bytes = Bytes::from_static(b"subscribe");
const TYPE_UNSUBSCRIBE: Bytes = Bytes::from_static(b"unsubscribe");
const TYPE_MESSAGE: Bytes = Bytes::from_static(b"message");

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

#[derive(Debug, PartialEq)]
pub enum SubscribeItem {
    Subscribed,
    UnSubscribed,
    Message(Bytes),
}

impl TryFrom<Response> for SubscribeItem {
    type Error = CommandError;

    fn try_from(val: Response) -> Result<Self, Self::Error> {
        let parts = if let Response::Array(ref parts) = val {
            parts
        } else {
            return Err(CommandError::Output("Cannot parse response", val));
        };

        if parts.len() != 3 {
            return Err(CommandError::Output(
                "Subscription message has invalid items number",
                val,
            ));
        }

        let (mtype, payload) = (&parts[0], &parts[2]);

        let mtype = if let Response::Bytes(mtype) = mtype {
            mtype
        } else {
            return Err(CommandError::Output(
                "Subscription message type unknown",
                val,
            ));
        };

        if mtype == &TYPE_SUBSCRIBE {
            return Ok(SubscribeItem::Subscribed);
        }

        if mtype == &TYPE_UNSUBSCRIBE {
            return Ok(SubscribeItem::UnSubscribed);
        }

        if mtype != &TYPE_MESSAGE {
            return Err(CommandError::Output(
                "Subscription message type unknown",
                val,
            ));
        }

        if let Response::Bytes(payload) = payload {
            return Ok(SubscribeItem::Message(payload.clone()));
        } else {
            return Err(CommandError::Output(
                "Subscription message has empty payload",
                val,
            ));
        }
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
pub fn UnSubscribe<T>(key: T) -> SubscribeOutputCommand
where
    BulkString: From<T>,
{
    SubscribeOutputCommand(Request::Array(vec![
        Request::from_static("UNSUBSCRIBE"),
        Request::BulkString(key.into()),
    ]))
}
