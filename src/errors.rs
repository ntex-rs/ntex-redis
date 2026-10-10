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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error() {
        let err = Error::Parse("bad".into());
        assert_eq!(err.to_string(), "Redis server response error: bad");
        assert!(matches!(err.clone(), Error::Parse(s) if s.is_empty()));

        let err = Error::from(io::Error::other("io"));
        assert!(matches!(&err, Error::PeerGone(Some(e)) if e.to_string() == "io"));
        assert!(err.to_string().starts_with("Io error: Some("));
        assert!(matches!(err.clone(), Error::PeerGone(None)));

        let err = Error::from(Either::Left(Error::PeerGone(None)));
        assert!(matches!(err, Error::PeerGone(None)));
        let err = Error::from(Either::<Error, _>::Right(io::Error::other("io")));
        assert!(matches!(err, Error::PeerGone(Some(_))));
    }

    #[test]
    fn test_command_error() {
        let err = CommandError::from(ByteString::from_static("ERR failed"));
        assert!(matches!(&err, CommandError::Error(e) if e == "ERR failed"));
        assert_eq!(err.to_string(), "ERR failed");

        let err = CommandError::from(("Unexpected value", Response::Nil));
        assert!(matches!(
            err,
            CommandError::Output("Unexpected value", Response::Nil)
        ));
        assert_eq!(
            err.to_string(),
            "Command output parse error: Unexpected value"
        );

        let err = CommandError::from(Either::Left(Error::Parse("bad".into())));
        assert!(matches!(err, CommandError::Protocol(Error::Parse(_))));
        assert_eq!(err.to_string(), "Redis server response error: bad");
        let err = CommandError::from(Either::<Error, _>::Right(io::Error::other("io")));
        assert!(matches!(
            err,
            CommandError::Protocol(Error::PeerGone(Some(_)))
        ));
    }

    #[test]
    fn test_connect_error() {
        assert_eq!(ConnectError::Unauthorized.to_string(), "Unauthorized");

        let err = ConnectError::from(CommandError::Error(ByteString::from_static("ERR")));
        assert!(matches!(
            &err,
            ConnectError::Command(CommandError::Error(_))
        ));
        assert_eq!(err.to_string(), "ERR");
    }
}
