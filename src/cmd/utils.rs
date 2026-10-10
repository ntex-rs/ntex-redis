use ntex_bytes::Bytes;
use std::convert::TryFrom;

use super::{Command, CommandError};
use crate::codec::{Request, Response};

/// Generic command that returns an optional bulk string
///
/// Output is `None` if the value does not exist.
pub struct BulkOutputCommand(pub(crate) Request);

impl Command for BulkOutputCommand {
    type Output = Option<Bytes>;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(None),
            Response::Bytes(val) => Ok(Some(val)),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}

/// Generic command that returns an integer
pub struct IntOutputCommand(pub(crate) Request);

impl Command for IntOutputCommand {
    type Output = i64;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Integer(val) => Ok(val),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}

/// Generic command that returns a boolean
///
/// Redis integer reply `1` is converted to `true`, `0` to `false`.
pub struct BoolOutputCommand(pub(crate) Request);

impl Command for BoolOutputCommand {
    type Output = bool;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        Ok(bool::try_from(val)?)
    }
}
