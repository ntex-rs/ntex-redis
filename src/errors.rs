use std::io;

use bytestring::ByteString;
use derive_more::{Display, From};
use ntex::connect;

#[derive(Debug, Display)]
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

#[derive(Debug, Display, From)]
pub enum ConnectError {
    Unauthorized,
    Command(CommandError),
    Connect(connect::ConnectError),
}

impl std::error::Error for ConnectError {}

#[derive(Debug, Display)]
pub enum CommandError {
    /// A redis server error response
    Error(ByteString),

    /// A command response parse error
    #[display(fmt = "Command output error: {}", _0)]
    Output(&'static str, crate::Response),

    /// Redis protocol level errors
    Protocol(Error),
}

impl std::error::Error for CommandError {}
