use super::{utils, Command, CommandError};
use ntex::util::Bytes;

use crate::codec::{BulkString, Request, Response};

const TYPE_SUBSCRIBE: Bytes = Bytes::from_static(b"subscribe");
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
    Message(Bytes),
}

pub struct SubscribeOutputCommand(pub(crate) Request);

impl Command for SubscribeOutputCommand {
    type Output = SubscribeItem;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Array(ref v) => match v.get(0) {
                Some(Response::Bytes(t)) => {
                    if t == &TYPE_SUBSCRIBE {
                        return Ok(SubscribeItem::Subscribed);
                    }
                    if t == &TYPE_MESSAGE {
                        return match v.get(2) {
                            Some(payload) => match payload {
                                Response::Bytes(m) => Ok(SubscribeItem::Message(m.clone())),
                                _ => {
                                    Err(CommandError::Output("Cannot parse message payload", val))
                                }
                            },
                            _ => Err(CommandError::Output("Empty messsage payload", val)),
                        };
                    }

                    Err(CommandError::Output("Unknown message type", val))
                }
                _ => Err(CommandError::Output("Cannot parse message type", val)),
            },
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
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
