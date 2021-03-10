//! Redis protocol related errors
use std::io;

use derive_more::{Display, From};
use ntex::{connect, util::ByteString, util::Either};

use super::codec::Response;

#[derive(Debug, Display)]
/// Redis protocol errors
pub enum Error {
    /// A RESP parsing error occurred
    #[display(fmt = "Redis server response error: {}", _0)]
    Parse(String),

    /// An IO error occurred
    #[display(fmt = "Io error: {:?}", _0)]
    Io(Option<io::Error>),

    /// Connection is disconnected
    #[display(fmt = "Redis server has been disconnected")]
    Disconnected,
}

impl std::error::Error for Error {}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Parse(_) => Error::Parse(String::new()),
            Error::Io(_) => Error::Io(None),
            Error::Disconnected => Error::Disconnected,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(Some(err))
    }
}

impl From<Either<Error, io::Error>> for Error {
    fn from(err: Either<Error, io::Error>) -> Error {
        match err {
            Either::Left(err) => err,
            Either::Right(err) => Error::Io(Some(err)),
        }
    }
}

#[derive(Debug, Display, From)]
/// Redis connectivity errors
pub enum ConnectError {
    /// Auth command failed
    Unauthorized,
    /// Command execution error
    Command(CommandError),
    /// Io connectivity error
    Connect(connect::ConnectError),
}

impl std::error::Error for ConnectError {}

#[derive(Debug, Display, From)]
/// Redis command execution errors
pub enum CommandError {
    /// A redis server error response
    Error(ByteString),

    /// A command response parse error
    #[display(fmt = "Command output error: {}", _0)]
    Output(&'static str, Response),

    /// Redis protocol level errors
    Protocol(Error),
}

impl std::error::Error for CommandError {}

impl From<Either<Error, io::Error>> for CommandError {
    fn from(err: Either<Error, io::Error>) -> CommandError {
        Into::<Error>::into(err).into()
    }
}
