use std::task::Poll;

use ntex::{io::IoBoxed, io::RecvError, util::poll_fn, util::ready};

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

        poll_fn(|cx| loop {
            return match ready!(self.io.poll_recv(&Codec, cx)) {
                Ok(item) => Poll::Ready(U::to_output(
                    item.into_result().map_err(CommandError::Error)?,
                )),
                Err(RecvError::KeepAlive) | Err(RecvError::Stop) => {
                    unreachable!()
                }
                Err(RecvError::WriteBackpressure) => {
                    ready!(self.io.poll_flush(cx, false))
                        .map_err(|e| CommandError::Protocol(Error::PeerGone(Some(e))))?;
                    continue;
                }
                Err(RecvError::Decoder(err)) => Poll::Ready(Err(CommandError::Protocol(err))),
                Err(RecvError::PeerGone(err)) => {
                    Poll::Ready(Err(CommandError::Protocol(Error::PeerGone(err))))
                }
            };
        })
        .await
    }

    pub(crate) fn into_inner(self) -> IoBoxed {
        self.io
    }
}
