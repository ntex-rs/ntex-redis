use super::{Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// Create DEL redis command
pub fn Del<T>(key: T) -> KeysCommand
where
    BulkString: From<T>,
{
    KeysCommand(vec![
        Request::from_static("DEL"),
        Request::BulkString(key.into()),
    ])
}

/// Create EXISTS redis command
pub fn Exists<T>(key: T) -> KeysCommand
where
    BulkString: From<T>,
{
    KeysCommand(vec![
        Request::from_static("EXISTS"),
        Request::BulkString(key.into()),
    ])
}

pub struct KeysCommand(Vec<Request>);

impl KeysCommand {
    /// Add a key to this command.
    pub fn key<T>(mut self, other: T) -> Self
    where
        BulkString: From<T>,
    {
        self.0.push(other.into());
        self
    }

    /// Add more keys to this command.
    pub fn keys<T>(mut self, other: impl IntoIterator<Item = T>) -> Self
    where
        BulkString: From<T>,
    {
        self.0.extend(other.into_iter().map(|t| t.into()));
        self
    }
}

impl Command for KeysCommand {
    type Output = usize;

    fn to_request(self) -> Request {
        Request::Array(self.0)
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Integer(val) => Ok(val as usize),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}
