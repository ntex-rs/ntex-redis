use std::io;

use bytestring::ByteString;
use derive_more::From;
use ntex::connect;

#[derive(Debug)]
pub enum Error {
    /// A RESP parsing error occurred
    Parse(String),

    /// An IO error occurred
    Io(Option<io::Error>),

    /// Connection is disconnected
    Disconnected,
}

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

#[derive(Debug, From)]
pub enum ConnectError {
    Unauthorized,
    Command(CommandError),
    Connect(connect::ConnectError),
}

#[derive(Debug)]
pub enum CommandError {
    /// A redis server error response
    Error(ByteString),

    /// A command response parse error
    Output(&'static str, crate::Response),

    /// Redis protocol level errors
    Protocol(Error),
}
