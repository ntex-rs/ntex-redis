use bytes::Bytes;

use super::{Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// LINDEX redis command
///
/// Returns the element at index index in the list stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng};
/// # use rand::distributions::Alphanumeric;
/// # fn gen_random_key() -> String {
/// # let key: String = thread_rng().sample_iter(&Alphanumeric).take(12).collect();
/// # key
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create list with one value
///     redis.exec(cmd::LPush(&key, "value"));
///
///     // get value by index
///     let value = redis.exec(cmd::LIndex(&key, 0)).await?;
///
///     assert_eq!(value.unwrap(), "value");
///     Ok(())
/// }
/// ```
pub fn LIndex<T>(key: T, index: i64) -> LIndexCommand
where
    BulkString: From<T>,
{
    LIndexCommand(Request::Array(vec![
        Request::from_static("LINDEX"),
        Request::BulkString(key.into()),
        Request::BulkInteger(index),
    ]))
}

pub struct LIndexCommand(Request);

impl Command for LIndexCommand {
    type Output = Option<Bytes>;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(None),
            Response::Bytes(val) => Ok(Some(val)),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}

/// LPOP redis command
///
/// Removes and returns the first element of the list stored at key.
pub fn LPop<T>(key: T) -> LPopCommand
where
    BulkString: From<T>,
{
    LPopCommand(Request::Array(vec![
        Request::from_static("LPOP"),
        Request::BulkString(key.into()),
    ]))
}

/// RPOP redis command
///
/// Removes and returns the last element of the list stored at key.
pub fn RPop<T>(key: T) -> LPopCommand
where
    BulkString: From<T>,
{
    LPopCommand(Request::Array(vec![
        Request::from_static("RPOP"),
        Request::BulkString(key.into()),
    ]))
}

pub struct LPopCommand(Request);

impl Command for LPopCommand {
    type Output = Option<Bytes>;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(None),
            Response::Bytes(val) => Ok(Some(val)),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}

/// LPUSH redis command
///
/// Insert all the specified values at the head of the list stored at key.
pub fn LPush<T, V>(key: T, value: V) -> LPushCommand
where
    BulkString: From<T> + From<V>,
{
    LPushCommand(vec![
        Request::from_static("LPUSH"),
        Request::BulkString(key.into()),
        Request::BulkString(value.into()),
    ])
}

/// RPUSH redis command
///
/// Insert all the specified values at the tail of the list stored at key.
pub fn RPush<T, V>(key: T, value: V) -> LPushCommand
where
    BulkString: From<T> + From<V>,
{
    LPushCommand(vec![
        Request::from_static("RPUSH"),
        Request::BulkString(key.into()),
        Request::BulkString(value.into()),
    ])
}

pub struct LPushCommand(Vec<Request>);

impl LPushCommand {
    /// Inserts specified values only if key already exists and holds a list.
    pub fn if_exists(mut self) -> Self {
        let cmd = if let Request::BulkStatic(ref s) = self.0[0] {
            if s == b"LPUSH" || s == b"LPUSHX" {
                "LPUSHX"
            } else {
                "RPUSHX"
            }
        } else {
            "LPUSHX"
        };
        self.0[0] = Request::from_static(cmd);
        self
    }
}

impl Command for LPushCommand {
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
