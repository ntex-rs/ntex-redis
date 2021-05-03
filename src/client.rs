use std::collections::VecDeque;
use std::{cell::RefCell, fmt, future::Future, pin::Pin, rc::Rc, task::Context, task::Poll};

use ntex::util::{poll_fn, Either, Ready};
use ntex::{channel::pool, framed::State, service::Service};

use super::cmd::Command;
use super::codec::{Codec, Request, Response};
use super::errors::{CommandError, Error};

#[derive(Clone)]
/// Shared redis client
pub struct Client(Rc<Inner>);

struct Inner {
    state: State,
    queue: RefCell<VecDeque<pool::Sender<Result<Response, Error>>>>,
    pool: pool::Pool<Result<Response, Error>>,
}

impl Client {
    pub(crate) fn new(state: State) -> Self {
        let inner = Rc::new(Inner {
            state,
            pool: pool::new(),
            queue: RefCell::new(VecDeque::new()),
        });
        let inner2 = inner.clone();

        // read redis response task
        ntex::rt::spawn(async move {
            let read = inner.state.read();

            poll_fn(|cx| {
                loop {
                    match read.decode(&Codec) {
                        Err(e) => {
                            if let Some(tx) = inner.queue.borrow_mut().pop_front() {
                                let _ = tx.send(Err(e));
                            }
                            inner.queue.borrow_mut().clear();
                            inner.state.shutdown_io();
                            return Poll::Ready(());
                        }
                        Ok(Some(item)) => {
                            if let Some(tx) = inner.queue.borrow_mut().pop_front() {
                                let _ = tx.send(Ok(item));
                            } else {
                                log::error!("Unexpected redis response: {:?}", item);
                            }
                        }
                        Ok(None) => break,
                    }
                }

                if !inner.state.is_open() {
                    return Poll::Ready(());
                }
                inner.state.register_dispatcher(cx.waker());
                Poll::Pending
            })
            .await
        });

        Client(inner2)
    }

    /// Execute redis command
    pub fn exec<T>(&self, cmd: T) -> impl Future<Output = Result<T::Output, CommandError>>
    where
        T: Command,
    {
        let is_open = self.0.state.is_open();
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
        self.0.state.is_open()
    }
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Either<CommandResult, Ready<Response, Error>>;

    fn poll_ready(&self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if !self.0.state.is_open() {
            Poll::Ready(Err(Error::Disconnected))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&self, req: Request) -> Self::Future {
        if let Err(e) = self.0.state.write().encode(req, &Codec) {
            Either::Right(Ready::Err(e))
        } else {
            let (tx, rx) = self.0.pool.channel();
            self.0.queue.borrow_mut().push_back(tx);
            Either::Left(CommandResult { rx })
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("connected", &self.0.state.is_open())
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
