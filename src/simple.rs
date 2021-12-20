use std::task::Poll;

use ntex::{io::IoBoxed, util::poll_fn, util::ready, util::Either};

use super::cmd::Command;
use super::codec::Codec;
use super::errors::{CommandError, Error};

/// Redis client
pub struct SimpleClient {
    io: IoBoxed,
}

impl SimpleClient {
    /// Create new simple client
    pub(crate) fn new(io: IoBoxed) -> Self {
        SimpleClient { io }
    }

    /// Execute redis command
    pub async fn exec<U>(&self, cmd: U) -> Result<U::Output, CommandError>
    where
        U: Command,
    {
        self.io.encode(cmd.to_request(), &Codec)?;

        poll_fn(|cx| match ready!(self.io.poll_read_next(&Codec, cx)) {
            Some(Ok(item)) => Poll::Ready(U::to_output(
                item.into_result().map_err(CommandError::Error)?,
            )),
            Some(Err(Either::Left(err))) => Poll::Ready(Err(CommandError::Protocol(err))),
            Some(Err(Either::Right(err))) => Poll::Ready(Err(CommandError::Protocol(err.into()))),
            None => Poll::Ready(Err(CommandError::Protocol(Error::Disconnected))),
        })
        .await
    }
}
