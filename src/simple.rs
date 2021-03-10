use futures::{SinkExt, StreamExt};
use ntex::codec::{AsyncRead, AsyncWrite, Framed};

use super::cmd::Command;
use super::codec::Codec;
use super::errors::{CommandError, Error};

/// Redis client
pub struct SimpleClient<T> {
    framed: Framed<T, Codec>,
}

impl<T> SimpleClient<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(framed: Framed<T, Codec>) -> Self {
        SimpleClient { framed }
    }

    /// Get reference to inner io object
    pub fn get_ref(&self) -> &Framed<T, Codec> {
        &self.framed
    }

    /// Get mut reference to inner io object
    pub fn get_mut(&mut self) -> &mut Framed<T, Codec> {
        &mut self.framed
    }

    /// Execute redis command
    pub async fn exec<U>(&mut self, cmd: U) -> Result<U::Output, CommandError>
    where
        U: Command,
    {
        self.framed.send(cmd.to_request()).await?;
        self.framed
            .next()
            .await
            .ok_or_else(|| CommandError::Protocol(Error::Disconnected))?
            .map_err(Into::into)
            .and_then(|res| U::to_output(res.into_result().map_err(CommandError::Error)?))
    }

    /// Return inner framed object
    pub fn into_inner(self) -> Framed<T, Codec> {
        self.framed
    }
}
