use std::pin::Pin;
use std::task::{Context, Poll};

use super::cmd::Command;
use super::codec::Codec;
use super::errors::{CommandError, Error};
use ntex::{io::IoBoxed, io::RecvError, util::poll_fn, util::ready, util::Stream};

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

    /// Execute redis command and stream response
    pub fn stream<U>(self, cmd: U) -> Result<RedisStream<U>, CommandError>
    where
        U: Command,
    {
        self.io.encode(cmd.to_request(), &Codec)?;

        Ok(RedisStream {
            io: self.io,
            _cmd: std::marker::PhantomData,
        })
    }

    pub(crate) fn into_inner(self) -> IoBoxed {
        self.io
    }
}

pub struct RedisStream<U: Command> {
    io: IoBoxed,
    _cmd: std::marker::PhantomData<U>,
}

impl<U: Command> Stream for RedisStream<U> {
    type Item = Result<U::Output, CommandError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

impl<U: Command> RedisStream<U> {
    /// Attempt to pull out the next value of this stream.
    pub async fn recv(&self) -> Option<Result<U::Output, CommandError>> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    /// Attempt to pull out the next value of this stream, registering
    /// the current task for wakeup if the value is not yet available,
    /// and returning None if the payload is exhausted.
    pub fn poll_recv(
        &self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<U::Output, CommandError>>> {
        match ready!(self.io.poll_recv(&Codec, cx)) {
            Ok(item) => match item.into_result() {
                Ok(result) => Poll::Ready(Some(U::to_output(result))),
                Err(err) => Poll::Ready(Some(Err(CommandError::Error(err)))),
            },
            Err(RecvError::KeepAlive) | Err(RecvError::Stop) => {
                unreachable!()
            }
            Err(RecvError::WriteBackpressure) => {
                if let Err(err) = ready!(self.io.poll_flush(cx, false))
                    .map_err(|e| CommandError::Protocol(Error::PeerGone(Some(e))))
                {
                    Poll::Ready(Some(Err(err)))
                } else {
                    Poll::Pending
                }
            }
            Err(RecvError::Decoder(err)) => Poll::Ready(Some(Err(CommandError::Protocol(err)))),
            Err(RecvError::PeerGone(err)) => {
                Poll::Ready(Some(Err(CommandError::Protocol(Error::PeerGone(err)))))
            }
        }
    }
}
