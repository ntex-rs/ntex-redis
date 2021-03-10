use super::{utils, Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// GET redis command
pub fn Get<T>(key: T) -> utils::BulkOutputCommand
where
    BulkString: From<T>,
{
    utils::BulkOutputCommand(Request::Array(vec![
        Request::from_static("GET"),
        Request::BulkString(key.into()),
    ]))
}

/// SET redis command
///
/// Set key to hold the string value. Command returns true if value is set
/// otherwise it returns false
pub fn Set<T, V>(key: T, value: V) -> SetCommand
where
    BulkString: From<T> + From<V>,
{
    SetCommand {
        req: vec![
            Request::from_bstatic(b"SET"),
            Request::BulkString(key.into()),
            Request::BulkString(value.into()),
        ],
        expire: Expire::None,
        keepttl: false,
        exists: None,
    }
}

enum Expire {
    None,
    Ex(Request),
    Px(Request),
}

pub struct SetCommand {
    req: Vec<Request>,
    expire: Expire,
    keepttl: bool,
    exists: Option<bool>,
}

impl SetCommand {
    /// Set the specified expire time, in seconds.
    pub fn expire_secs(mut self, secs: i64) -> Self {
        self.expire = Expire::Ex(Request::BulkInteger(secs));
        self
    }

    /// Set the specified expire time, in milliseconds.
    pub fn expire_millis(mut self, secs: i64) -> Self {
        self.expire = Expire::Px(Request::BulkInteger(secs));
        self
    }

    /// Only set the key if it already exist.
    pub fn if_exists(mut self) -> Self {
        self.exists = Some(true);
        self
    }

    /// Only set the key if it does not already exist.
    pub fn if_not_exists(mut self) -> Self {
        self.exists = Some(false);
        self
    }

    /// Retain the time to live associated with the key.
    pub fn keepttl(mut self) -> Self {
        self.keepttl = true;
        self
    }
}

impl Command for SetCommand {
    type Output = bool;

    fn to_request(mut self) -> Request {
        // EX|PX
        match self.expire {
            Expire::None => (),
            Expire::Ex(r) => {
                self.req.push(Request::from_bstatic(b"EX"));
                self.req.push(r);
            }
            Expire::Px(r) => {
                self.req.push(Request::from_bstatic(b"PX"));
                self.req.push(r);
            }
        }

        // NX|XX
        if let Some(exists) = self.exists {
            self.req.push(if exists {
                Request::from_bstatic(b"XX")
            } else {
                Request::from_bstatic(b"NX")
            });
        }

        // KEEPTTL
        if self.keepttl {
            self.req.push(Request::from_bstatic(b"KEEPTTL"))
        }

        Request::Array(self.req)
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(false),
            Response::String(string) => match string.as_ref() {
                "OK" => Ok(true),
                _ => Err(CommandError::Output(
                    "Unexpected value within String",
                    Response::String(string),
                )),
            },
            _ => Err(CommandError::Output("Unexpected value", val)),
        }
    }
}

/// INCRBY redis command
///
/// Increments the number stored at `key` by `increment`.
pub fn IncrBy<T, I>(key: T, increment: I) -> utils::IntOutputCommand
where
    BulkString: From<T>,
    i64: From<I>,
{
    utils::IntOutputCommand(Request::Array(vec![
        Request::from_static("INCRBY"),
        Request::BulkString(key.into()),
        Request::BulkString(i64::from(increment).to_string().into()),
    ]))
}
