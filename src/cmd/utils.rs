use ntex_bytes::Bytes;

use super::{Command, CommandError};
use crate::codec::{Args, Frame, Writer};

/// Generic command that returns an optional bulk string
///
/// Output is `None` if the value does not exist.
pub struct BulkOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for BulkOutputCommand<A> {
    type Output = Option<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.bulk()
    }
}

/// Generic command that returns an integer
pub struct IntOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for IntOutputCommand<A> {
    type Output = i64;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.int()
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

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        match frame.int()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(frame.unexpected("i64 value cannot be represented as bool")),
        }
    }
}
