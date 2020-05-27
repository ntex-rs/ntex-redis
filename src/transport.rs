use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Future, Stream};
use ntex::channel::{mpsc, pool};
use ntex::codec::{AsyncRead, AsyncWrite, Framed};

use super::codec::{Codec, Request, Response};
use super::errors::Error;

type Rx = mpsc::Receiver<(Request, pool::Sender<Result<Response, Error>>)>;

pub(super) struct Transport<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    rx: Rx,
    framed: Framed<T, Codec>,
    queue: VecDeque<pool::Sender<Result<Response, Error>>>,
}

impl<T> Transport<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(rx: Rx, framed: Framed<T, Codec>) -> Self {
        Transport {
            rx,
            framed,
            queue: VecDeque::with_capacity(16),
        }
    }

    fn failed(&mut self, err: Error) {
        for tx in self.queue.drain(..) {
            let _ = tx.send(Err(err.clone()));
        }
    }
}

impl<T> Future for Transport<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // write
        loop {
            while !self.framed.is_write_buf_full() {
                match Pin::new(&mut self.rx).poll_next(cx) {
                    Poll::Pending => break,
                    Poll::Ready(Some((req, tx))) => {
                        self.queue.push_back(tx);
                        if let Err(e) = self.framed.write(req) {
                            self.failed(e);
                            return Poll::Ready(());
                        }
                    }
                    Poll::Ready(None) => return Poll::Ready(()),
                }
            }

            // flush
            if !self.framed.is_write_buf_empty() {
                match self.framed.flush(cx) {
                    Poll::Pending => break,
                    Poll::Ready(Ok(_)) => (),
                    Poll::Ready(Err(err)) => {
                        log::debug!("Error sending data: {:?}", err);
                        self.failed(err);
                        return Poll::Ready(());
                    }
                }
            } else {
                break;
            }
        }

        // read redis responses
        loop {
            match self.framed.next_item(cx) {
                Poll::Ready(Some(Ok(el))) => {
                    if let Some(tx) = self.queue.pop_front() {
                        let _ = tx.send(Ok(el));
                    } else {
                        log::error!("Unexpected redis response: {:?}", el);
                    }
                }
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Err(err))) => {
                    log::trace!("Client disconnected with error: {:?}", err);
                    self.failed(err);
                    return Poll::Ready(());
                }
                Poll::Ready(None) => {
                    log::trace!("Client disconnected");
                    return Poll::Ready(());
                }
            }
        }
    }
}
