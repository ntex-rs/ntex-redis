use std::{fmt, future::Future, pin::Pin, task::Context, task::Poll};

use ntex::channel::{mpsc, pool};
use ntex::service::Service;

use super::cmd::Command;
use super::codec::{Request, Response};
use super::errors::{CommandError, Error};

#[derive(Clone)]
/// Shared redis client
pub struct Client {
    pool: pool::Pool<Result<Response, Error>>,
    transport: mpsc::Sender<(Request, pool::Sender<Result<Response, Error>>)>,
}

impl Client {
    pub(crate) fn new(
        transport: mpsc::Sender<(Request, pool::Sender<Result<Response, Error>>)>,
    ) -> Self {
        Client {
            transport,
            pool: pool::new(),
        }
    }

    /// Execute redis command
    pub fn exec<T>(&self, cmd: T) -> impl Future<Output = Result<T::Output, CommandError>>
    where
        T: Command,
    {
        let fut = self.call(cmd.to_request());

        async move {
            fut.await
                .map_err(CommandError::Protocol)
                .and_then(|res| T::to_output(res.into_result().map_err(CommandError::Error)?))
        }
    }

    /// Delete all the keys of the currently selected DB.
    pub async fn flushdb(&self) -> Result<(), Error> {
        self.call("FLUSHDB".into()).await?;
        Ok(())
    }

    /// Returns false if underlying transport is disconnected from Redis
    pub fn is_connected(&self) -> bool {
        !self.transport.is_closed()
    }
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = CommandResult;

    fn poll_ready(&self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.transport.is_closed() {
            Poll::Ready(Err(Error::Disconnected))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&self, req: Request) -> Self::Future {
        let (tx, rx) = self.pool.channel();
        let _ = self.transport.send((req, tx));
        CommandResult { rx }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("connected", &!self.transport.is_closed())
            .finish()
    }
}

pub struct CommandResult {
    rx: pool::Receiver<Result<Response, Error>>,
}

impl Future for CommandResult {
    type Output = Result<Response, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(res)) => Poll::Ready(res),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::Disconnected)),
            Poll::Pending => Poll::Pending,
        }
    }
}
