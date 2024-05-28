use std::collections::VecDeque;
use std::{cell::RefCell, fmt, future::poll_fn, rc::Rc, task::Poll};

use ntex::io::{IoBoxed, IoRef, OnDisconnect, RecvError};
use ntex::util::ready;
use ntex::{channel::pool, service::Service, service::ServiceCtx};

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
                match ready!(io.poll_recv(&Codec, cx)) {
                    Ok(item) => {
                        if let Some(tx) = queue2.borrow_mut().pop_front() {
                            let _ = tx.send(Ok(item));
                        } else {
                            log::error!("Unexpected redis response: {:?}", item);
                        }
                        continue;
                    }
                    Err(RecvError::KeepAlive) | Err(RecvError::Stop) => {
                        unreachable!()
                    }
                    Err(RecvError::WriteBackpressure) => {
                        if ready!(io.poll_flush(cx, false)).is_err() {
                            return Poll::Ready(());
                        } else {
                            continue;
                        }
                    }
                    Err(RecvError::Decoder(e)) => {
                        if let Some(tx) = queue2.borrow_mut().pop_front() {
                            let _ = tx.send(Err(e));
                        }
                        queue2.borrow_mut().clear();
                        let _ = ready!(io.poll_shutdown(cx));
                        return Poll::Ready(());
                    }
                    Err(RecvError::PeerGone(e)) => {
                        log::info!("Redis connection is dropped: {:?}", e);
                        queue2.borrow_mut().clear();
                        return Poll::Ready(());
                    }
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
    pub async fn exec<T>(&self, cmd: T) -> Result<T::Output, CommandError>
    where
        T: Command,
    {
        if self.io.is_closed() {
            Err(CommandError::Protocol(Error::PeerGone(None)))
        } else {
            self._call(cmd.to_request())
                .await
                .map_err(CommandError::Protocol)
                .and_then(|res| T::to_output(res.into_result().map_err(CommandError::Error)?))
        }
    }

    /// Delete all the keys of the currently selected DB.
    pub async fn flushdb(&self) -> Result<(), Error> {
        self._call("FLUSHDB".into()).await?;
        Ok(())
    }

    /// Returns true if underlying transport is connected to redis
    pub fn is_connected(&self) -> bool {
        !self.io.is_closed()
    }

    async fn _call(&self, req: Request) -> Result<Response, Error> {
        if let Err(e) = self.io.encode(req, &Codec) {
            Err(e)
        } else {
            let (tx, rx) = self.pool.channel();
            self.queue.borrow_mut().push_back(tx);
            poll_fn(|cx| rx.poll_recv(cx))
                .await
                .map_err(|_| Error::PeerGone(None))
                .and_then(|v| v)
        }
    }
}

impl Service<Request> for Client {
    type Response = Response;
    type Error = Error;

    async fn ready(&self, _: ServiceCtx<'_, Self>) -> Result<(), Self::Error> {
        poll_fn(|cx| {
            if self.disconnect.poll_ready(cx).is_ready() {
                Poll::Ready(Err(Error::PeerGone(None)))
            } else {
                Poll::Ready(Ok(()))
            }
        })
        .await
    }

    async fn call(&self, req: Request, _: ServiceCtx<'_, Self>) -> Result<Response, Error> {
        self._call(req).await
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("connected", &!self.io.is_closed())
            .finish()
    }
}
