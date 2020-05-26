use bytes::Bytes;
use bytestring::ByteString;

use super::{Command, CommandError, Request, Response};

/// Create GET redis command
pub fn Get<T>(key: T) -> GetCommand
where
    Request: From<T>,
{
    GetCommand(Request::Array(vec![
        Request::String(ByteString::from_static("GET")),
        key.into(),
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
