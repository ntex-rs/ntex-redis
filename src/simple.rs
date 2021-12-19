use std::task::Poll;

use ntex::{io::IoBoxed, util::poll_fn};

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
    pub async fn exec<U>(&mut self, cmd: U) -> Result<U::Output, CommandError>
    where
        U: Command,
    {
        self.io.write().encode(cmd.to_request(), &Codec)?;

        let read = self.io.read();
        poll_fn(|cx| {
            if let Some(item) = read.decode(&Codec)? {
                return Poll::Ready(U::to_output(
                    item.into_result().map_err(CommandError::Error)?,
                ));
            }

            if let Err(err) = read.poll_read_ready(cx) {
                Poll::Ready(Err(CommandError::Protocol(Error::Io(err))))
            } else {
                Poll::Pending
            }
        })
        .await
    }
}
