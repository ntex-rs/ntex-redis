use super::{Command, CommandError};
use crate::codec::{Arg, Frame, Writer};

/// AUTH redis command
///
/// Authenticate the current connection. Returns `true` if the password is accepted.
///
/// Use [`RedisConnector::password`](crate::RedisConnector::password) to
/// authenticate new connections automatically.
pub fn Auth<T: Arg>(password: T) -> AuthCommand<T> {
    AuthCommand(password)
}

/// AUTH command, see [`Auth`](crate::cmd::Auth)
pub struct AuthCommand<T>(T);

impl<T: Arg> Command for AuthCommand<T> {
    type Output = bool;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(("AUTH", self.0));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.peek()? == b'+' && frame.string()? == "OK")
    }
}
