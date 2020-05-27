use bytes::Bytes;

use super::{Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// Create GET redis command
pub fn Get<T>(key: T) -> GetCommand
where
    BulkString: From<T>,
{
    GetCommand(Request::Array(vec![
        Request::from_static("GET"),
        Request::BulkString(key.into()),
    ]))
}

pub struct GetCommand(Request);

impl Command for GetCommand {
    type Output = Option<Bytes>;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(None),
            Response::Bytes(val) => Ok(Some(val)),
            Response::String(val) => Ok(Some(val.into_inner())),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}

/// Create SET redis command
///
/// Command returns true if value is set
/// otherwise it returns false
pub fn Set<T, V>(key: T, value: V) -> SetCommand
where
    BulkString: From<T> + From<V>,
{
    SetCommand(Request::Array(vec![
        Request::from_bstatic(b"SET"),
        Request::BulkString(key.into()),
        Request::BulkString(value.into()),
    ]))
}

pub struct SetCommand(Request);

impl Command for SetCommand {
    type Output = bool;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(false),
            Response::String(string) => match string.as_ref() {
                "OK" => Ok(true),
                _ => Err(CommandError::Output(
                    "Unexpected value within String",
                    Response::String(string),
                )),
            },
            _ => Err(CommandError::Output("Unexpected value", val)),
        }
    }
}
