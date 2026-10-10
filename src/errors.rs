//! Redis protocol related errors
use std::io;

use ntex_bytes::ByteString;
use ntex_net::connect;
use ntex_util::future::Either;

use super::codec::Response;

#[derive(Debug, thiserror::Error)]
/// Redis protocol errors
pub enum Error {
    /// Redis server response cannot be parsed (RESP protocol error)
    #[error("Redis server response error: {0}")]
    Parse(String),

    /// Connection is closed, with optional IO error
    #[error("Io error: {0:?}")]
    PeerGone(Option<io::Error>),
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Parse(_) => Error::Parse(String::new()),
            Error::PeerGone(_) => Error::PeerGone(None),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::PeerGone(Some(err))
    }
}

impl From<Either<Error, io::Error>> for Error {
    fn from(err: Either<Error, io::Error>) -> Error {
        match err {
            Either::Left(err) => err,
            Either::Right(err) => Error::PeerGone(Some(err)),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
/// Redis connectivity errors
pub enum ConnectError {
    /// Auth command failed
    #[error("Unauthorized")]
    Unauthorized,

    /// Command execution error
    #[error(transparent)]
    Command(#[from] CommandError),

    /// Io connectivity error
    #[error(transparent)]
    Connect(#[from] ntex_error::Error<connect::ConnectError>),
}

#[derive(Debug, Clone, thiserror::Error)]
/// Redis command execution errors
pub enum CommandError {
    /// A redis server error response
    #[error("{0}")]
    Error(ByteString),

    /// A command response parse error
    #[error("Command output parse error: {0}")]
    Output(&'static str, Response),

    /// Redis protocol level errors
    #[error(transparent)]
    Protocol(#[from] Error),
}

impl From<ByteString> for CommandError {
    fn from(err: ByteString) -> CommandError {
        CommandError::Error(err)
    }
}

impl From<(&'static str, Response)> for CommandError {
    fn from((msg, resp): (&'static str, Response)) -> CommandError {
        CommandError::Output(msg, resp)
    }
}

impl From<Either<Error, io::Error>> for CommandError {
    fn from(err: Either<Error, io::Error>) -> CommandError {
        Into::<Error>::into(err).into()
    }
}
