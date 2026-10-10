use ntex_bytes::Bytes;
use std::convert::TryFrom;

use super::{Command, CommandError};
use crate::codec::{Args, Response, Writer};

/// Generic command that returns an optional bulk string
///
/// Output is `None` if the value does not exist.
pub struct BulkOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for BulkOutputCommand<A> {
    type Output = Option<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
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
pub struct IntOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for IntOutputCommand<A> {
    type Output = i64;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
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
pub struct BoolOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for BoolOutputCommand<A> {
    type Output = bool;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        Ok(bool::try_from(val)?)
    }
}
