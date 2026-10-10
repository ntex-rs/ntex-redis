use std::pin::Pin;
use std::{future::poll_fn, task::Context, task::Poll, task::ready};

use super::cmd::{Command, commands::PubSubCommand, commands::SubscribeOutputCommand};
use super::codec::{Codec, Writer};
use super::errors::{CommandError, Error};
use ntex_io::{IoBoxed, RecvError};
use ntex_util::Stream;

/// Simple redis client
///
/// Executes one command at a time and waits for its response, without
/// pipelining. It is not clonable, but owns the connection, so it can be
/// switched to subscription mode with [`SimpleClient::subscribe`].
/// Use [`Client`](crate::Client) for a shared, pipelined connection.
///
/// Created by [`RedisConnector::connect_simple`](crate::RedisConnector::connect_simple).
pub struct SimpleClient {
    io: IoBoxed,
}

impl SimpleClient {
    /// Create new simple client
    pub(crate) fn new(io: IoBoxed) -> Self {
        SimpleClient { io }
    }

    /// Execute redis command and wait result
    pub async fn exec<U>(&self, cmd: U) -> Result<U::Output, CommandError>
    where
        U: Command,
    {
        self.send(cmd)?;
        loop {
            if let Some(result) = self.recv::<U>().await {
                return result;
            }
        }
    }

    /// Send redis command without waiting for the response
    ///
    /// The response is not read. Redis replies to every command outside of
    /// subscription mode, so the next [`exec`](SimpleClient::exec) call
    /// receives this response instead of its own. Prefer `exec`.
    pub fn send<U>(&self, cmd: U) -> Result<(), CommandError>
    where
        U: Command,
    {
        self.io
            .with_write_src(|buf| cmd.encode(&mut Writer::new(buf)))
            .map_err(|e| CommandError::Protocol(e.into()))
    }

    /// Execute subscribe command and switch the connection to subscription mode
    ///
    /// Accepts [`Subscribe`](crate::cmd::Subscribe),
    /// [`SSubscribe`](crate::cmd::SSubscribe) and [`PSubscribe`](crate::cmd::PSubscribe)
    /// commands. Returned client is a [`Stream`] of subscription items.
    pub fn subscribe(
        self,
        cmd: SubscribeOutputCommand,
    ) -> Result<SubscriptionClient<SubscribeOutputCommand>, CommandError> {
        self.send(cmd)?;
        Ok(SubscriptionClient {
            client: self,
            _cmd: std::marker::PhantomData,
        })
    }

    pub(crate) fn into_inner(self) -> IoBoxed {
        self.io
    }

    async fn recv<U: Command>(&self) -> Option<Result<U::Output, CommandError>> {
        poll_fn(|cx| self.poll_recv::<U>(cx)).await
    }

    fn poll_recv<U: Command>(
        &self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<U::Output, CommandError>>> {
        loop {
            return match ready!(self.io.poll_recv(&Codec, cx)) {
                Ok(item) => match item.into_result() {
                    Ok(result) => Poll::Ready(Some(U::to_output(result))),
                    Err(err) => Poll::Ready(Some(Err(CommandError::Error(err)))),
                },
                // client io does not use dispatcher timers
                Err(RecvError::Timeout) => continue,
                Err(RecvError::WriteBackpressure) => {
                    if let Err(err) = ready!(self.io.poll_flush(cx, false)) {
                        Poll::Ready(Some(Err(CommandError::Protocol(Error::PeerGone(Some(
                            err,
                        ))))))
                    } else {
                        continue;
                    }
                }
                Err(RecvError::Decoder(err)) => Poll::Ready(Some(Err(CommandError::Protocol(err)))),
                Err(RecvError::PeerGone(err)) => {
                    Poll::Ready(Some(Err(CommandError::Protocol(Error::PeerGone(err)))))
                }
            };
        }
    }
}

/// Redis pubsub client to receive push messages
///
/// Created by [`SimpleClient::subscribe`]. The stream does not terminate,
/// it yields errors after the connection is closed.
pub struct SubscriptionClient<U: Command + PubSubCommand> {
    client: SimpleClient,
    _cmd: std::marker::PhantomData<U>,
}

impl<U: Command + PubSubCommand> Stream for SubscriptionClient<U> {
    type Item = Result<U::Output, CommandError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

impl<U: Command + PubSubCommand> SubscriptionClient<U> {
    /// Get client back
    ///
    /// The connection stays in subscription mode, reset it with
    /// [`Reset`](crate::cmd::Reset) before executing regular commands.
    ///
    /// ```rust
    /// use ntex_redis::{cmd, RedisConnector};
    ///
    /// #[ntex::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let redis = RedisConnector::new("127.0.0.1:6379").connect_simple().await?;
    ///
    ///     let subscriber = redis.subscribe(cmd::Subscribe(vec!["test"]))?;
    ///     // do some work
    ///
    ///     // go back to normal client
    ///     let redis = subscriber.into_client();
    ///
    ///     // reset connection. Pending subscription messages may be
    ///     // received instead of the RESET response, which is an error
    ///     if let Err(e) = redis.exec(cmd::Reset()).await {
    ///         println!("Error on reset connection: {}", e);
    ///     };
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn into_client(self) -> SimpleClient {
        self.client
    }

    /// Send redis subscribe/unsubscribe command
    ///
    /// Confirmations are received as [`SubscribeItem`](crate::cmd::SubscribeItem)s
    /// from this stream.
    pub fn send<T: Command + PubSubCommand>(&self, cmd: T) -> Result<(), CommandError> {
        self.client.send(cmd)
    }

    /// Receive next item from this stream
    ///
    /// Returns an error if the connection is closed, never `None`.
    pub async fn recv(&self) -> Option<Result<U::Output, CommandError>> {
        poll_fn(|cx| self.client.poll_recv::<U>(cx)).await
    }

    /// Attempt to pull out the next value of this stream, registering
    /// the current task for wakeup if the value is not yet available.
    ///
    /// Returns an error if the connection is closed, never `None`.
    pub fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<Option<Result<U::Output, CommandError>>> {
        self.client.poll_recv::<U>(cx)
    }
}
