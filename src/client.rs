use std::collections::VecDeque;
use std::{cell::RefCell, fmt, future::Future, pin::Pin, rc::Rc, task::Context, task::Poll};

use ntex::io::{IoBoxed, IoRef, OnDisconnect};
use ntex::util::{poll_fn, ready, Either, Ready};
use ntex::{channel::pool, service::Service};

use super::cmd::Command;
use super::codec::{Codec, Request, Response};
use super::errors::{CommandError, Error};

type Queue = Rc<RefCell<VecDeque<pool::Sender<Result<Response, Error>>>>>;

#[derive(Clone)]
/// Shared redis client
pub struct Client {
    io: IoRef,
    queue: Queue,
    disconnect: OnDisconnect,
    pool: pool::Pool<Result<Response, Error>>,
}

impl Client {
    pub(crate) fn new(io: IoBoxed) -> Self {
        let queue: Queue = Rc::new(RefCell::new(VecDeque::new()));

        // read redis response task
        let io_ref = io.get_ref();
        let queue2 = queue.clone();
        ntex::rt::spawn(async move {
            poll_fn(|cx| loop {
                match ready!(io.poll_read_next(&Codec, cx)) {
                    Some(Ok(item)) => {
                        if let Some(tx) = queue2.borrow_mut().pop_front() {
                            let _ = tx.send(Ok(item));
                        } else {
                            log::error!("Unexpected redis response: {:?}", item);
                        }
                        continue;
                    }
                    Some(Err(Either::Left(e))) => {
                        if let Some(tx) = queue2.borrow_mut().pop_front() {
                            let _ = tx.send(Err(e));
                        }
                        queue2.borrow_mut().clear();
                        let _ = ready!(io.poll_shutdown(cx));
                        return Poll::Ready(());
                    }
                    Some(Err(Either::Right(e))) => {
                        if let Some(tx) = queue2.borrow_mut().pop_front() {
                            let _ = tx.send(Err(e.into()));
                        }
                        queue2.borrow_mut().clear();
                        let _ = ready!(io.poll_shutdown(cx));
                        return Poll::Ready(());
                    }
                    None => return Poll::Ready(()),
                }
            })
            .await
        });

        let disconnect = io_ref.on_disconnect();

        Client {
            queue,
            disconnect,
            io: io_ref,
            pool: pool::new(),
        }
    }

    /// Execute redis command
    pub fn exec<T>(&self, cmd: T) -> impl Future<Output = Result<T::Output, CommandError>>
    where
        T: Command,
    {
        let is_open = !self.io.is_closed();
        let fut = self.call(cmd.to_request());

        async move {
            if !is_open {
                Err(CommandError::Protocol(Error::Disconnected))
            } else {
                fut.await
                    .map_err(CommandError::Protocol)
                    .and_then(|res| T::to_output(res.into_result().map_err(CommandError::Error)?))
            }
        }
    }

    /// Delete all the keys of the currently selected DB.
    pub async fn flushdb(&self) -> Result<(), Error> {
        self.call("FLUSHDB".into()).await?;
        Ok(())
    }

    /// Returns true if underlying transport is connected to redis
    pub fn is_connected(&self) -> bool {
        !self.io.is_closed()
    }
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Either<CommandResult, Ready<Response, Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.disconnect.poll_ready(cx).is_ready() {
            Poll::Ready(Err(Error::Disconnected))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&self, req: Request) -> Self::Future {
        if let Err(e) = self.io.encode(req, &Codec) {
            Either::Right(Ready::Err(e))
        } else {
            let (tx, rx) = self.pool.channel();
            self.queue.borrow_mut().push_back(tx);
            Either::Left(CommandResult { rx })
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("connected", &!self.io.is_closed())
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
