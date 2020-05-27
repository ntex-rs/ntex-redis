use bytes::Bytes;

use super::{Command, CommandError};
use crate::codec::{Request, Response};

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

pub struct IntOutputCommand(pub(crate) Request);

impl Command for IntOutputCommand {
    type Output = usize;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Integer(val) => Ok(val as usize),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}
