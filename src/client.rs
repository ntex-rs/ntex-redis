use std::collections::VecDeque;
use std::{cell::RefCell, fmt, future::poll_fn, rc::Rc, task::Poll, task::ready};

use ntex_io::{IoBoxed, IoRef, RecvError};
use ntex_service::{Ctx, Service};
use ntex_util::channel::pool;

use super::cmd::Command;
use ntex_bytes::Bytes;

use super::codec::{Codec, Frame, FrameCodec, Request, Response, Writer};
use super::errors::{CommandError, Error};

type Queue = Rc<RefCell<VecDeque<pool::Sender<Result<Bytes, Error>>>>>;

/// Shared redis client
///
/// Client is cheap to clone, all clones share one connection. Commands are
/// pipelined: multiple commands can be in flight and responses are matched
/// in order. Pub/sub is not supported, use [`SimpleClient`](crate::SimpleClient).
///
/// Created by [`RedisConnector::connect`](crate::RedisConnector::connect).
#[derive(Clone)]
pub struct Client {
    io: IoRef,
    queue: Queue,
    pool: pool::Pool<Result<Bytes, Error>>,
}

impl Client {
    pub(crate) fn new(io: IoBoxed) -> Self {
        let queue: Queue = Rc::new(RefCell::new(VecDeque::new()));

        // read redis response task
        let io_ref = io.get_ref();
        let queue2 = queue.clone();
        ntex_util::spawn(async move {
            let codec = FrameCodec::default();
            poll_fn(|cx| {
                loop {
                    match ready!(io.poll_recv(&codec, cx)) {
                        Ok(item) => {
                            if let Some(tx) = queue2.borrow_mut().pop_front() {
                                let _ = tx.send(Ok(item));
                            } else {
                                log::error!("Unexpected redis response: {item:?}");
                            }
                            continue;
                        }
                        // client io does not use dispatcher timers
                        Err(RecvError::Timeout) => continue,
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
                            log::info!("Redis connection is dropped: {e:?}");
                            queue2.borrow_mut().clear();
                            return Poll::Ready(());
                        }
                    }
                }
            })
            .await
        });

        Client {
            queue,
            io: io_ref,
            pool: pool::new(),
        }
    }

    /// Execute redis command and wait for the response
    pub async fn exec<T>(&self, cmd: T) -> Result<T::Output, CommandError>
    where
        T: Command,
    {
        if self.io.is_closed() {
            Err(CommandError::Protocol(Error::PeerGone(None)))
        } else {
            self.send(|io| io.with_write_src(|buf| cmd.encode(&mut Writer::new(buf))))
                .await
                .map_err(CommandError::Protocol)
                .and_then(|frame| Frame::new(frame).decode::<T>())
        }
    }

    /// Delete all the keys of the currently selected DB.
    pub async fn flushdb(&self) -> Result<(), Error> {
        self._call(Request::Array(vec![Request::from_static("FLUSHDB")]))
            .await?;
        Ok(())
    }

    /// Returns true if underlying transport is connected to redis
    pub fn is_connected(&self) -> bool {
        !self.io.is_closed()
    }

    async fn _call(&self, req: Request) -> Result<Response, Error> {
        let frame = self.send(|io| io.encode(req, &Codec::new())).await?;
        Frame::new(frame).read_value()
    }

    async fn send<F, E>(&self, f: F) -> Result<Bytes, Error>
    where
        F: FnOnce(&IoRef) -> Result<(), E>,
        Error: From<E>,
    {
        if let Err(e) = f(&self.io) {
            Err(e.into())
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

impl<St> Service<St, Request> for Client {
    type Res = Response;
    type Error = Error;

    async fn ready(&self, _: Ctx<'_, Self, St>) -> Result<(), Self::Error> {
        if self.io.is_closed() {
            Err(Error::PeerGone(None))
        } else {
            Ok(())
        }
    }

    async fn call(&self, req: Request, _: Ctx<'_, Self, St>) -> Result<Response, Error> {
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
