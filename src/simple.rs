use std::task::Poll;

use ntex::{framed::State, util::poll_fn};

use super::cmd::Command;
use super::codec::Codec;
use super::errors::{CommandError, Error};

/// Redis client
pub struct SimpleClient {
    state: State,
}

impl SimpleClient {
    /// Create new simple client
    pub(crate) fn new(state: State) -> Self {
        SimpleClient { state }
    }

    /// Execute redis command
    pub async fn exec<U>(&mut self, cmd: U) -> Result<U::Output, CommandError>
    where
        U: Command,
    {
        self.state.write().encode(cmd.to_request(), &Codec)?;

        let read = self.state.read();
        poll_fn(|cx| {
            if let Some(item) = read.decode(&Codec)? {
                return Poll::Ready(U::to_output(
                    item.into_result().map_err(CommandError::Error)?,
                ));
            }

            if !self.state.is_open() {
                return Poll::Ready(Err(CommandError::Protocol(Error::Disconnected)));
            }

            self.state.register_dispatcher(cx.waker());
            Poll::Pending
        })
        .await
    }
}
