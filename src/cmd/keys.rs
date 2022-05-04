use std::convert::{TryFrom, TryInto};

use ntex::util::ByteString;

use super::{utils, Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// DEL redis command
///
/// Removes the specified keys. A key is ignored if it does not exist.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // set string value
///     redis.exec(cmd::Set(&key, "value")).await?;
///
///     // remove keys
///     let value = redis.exec(
///         cmd::Del(&key).key("test_1").key("test_2")
///     ).await?;
///
///     assert_eq!(value, 1);
///     Ok(())
/// }
/// ```
pub fn Del<T>(key: T) -> KeysCommand
where
    BulkString: From<T>,
{
    KeysCommand(vec![
        Request::from_static("DEL"),
        Request::BulkString(key.into()),
    ])
}

/// EXISTS redis command
///
/// Returns if key exists.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // set string value
///     redis.exec(cmd::Set(&key, "value")).await?;
///
///     // check keys existence
///     let value = redis.exec(
///         cmd::Exists(&key).keys(vec!["test_1", "test_2"])
///     ).await?;
///
///     assert_eq!(value, 1);
///     Ok(())
/// }
/// ```
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

/// EXPIRE redis command
///
/// Set a timeout on `key`.
pub fn Expire<T, S>(key: T, seconds: S) -> utils::BoolOutputCommand
where
    BulkString: From<T>,
    i64: From<S>,
{
    utils::BoolOutputCommand(Request::Array(vec![
        Request::from_static("EXPIRE"),
        Request::BulkString(key.into()),
        Request::BulkString(i64::from(seconds).to_string().into()),
    ]))
}

/// EXPIREAT redis command
///
/// Set a timeout on `key`.
pub fn ExpireAt<T, S>(key: T, timestamp: S) -> utils::BoolOutputCommand
where
    BulkString: From<T>,
    i64: From<S>,
{
    utils::BoolOutputCommand(Request::Array(vec![
        Request::from_static("EXPIREAT"),
        Request::BulkString(key.into()),
        Request::BulkString(i64::from(timestamp).to_string().into()),
    ]))
}

/// TTL redis command
///
/// Returns the remaining time to live of a `key` that has a timeout.
pub fn Ttl<T>(key: T) -> TtlCommand
where
    BulkString: From<T>,
{
    TtlCommand(vec![
        Request::from_static("TTL"),
        Request::BulkString(key.into()),
    ])
}

#[derive(Debug, PartialEq)]
pub enum TtlResult {
    Seconds(i64),
    NoExpire,
    NotFound,
}

pub struct TtlCommand(Vec<Request>);

impl Command for TtlCommand {
    type Output = TtlResult;

    fn to_request(self) -> Request {
        Request::Array(self.0)
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        let result = i64::try_from(val)?;
        Ok(match result {
            -1 => TtlResult::NoExpire,
            -2 => TtlResult::NotFound,
            s => TtlResult::Seconds(s),
        })
    }
}

/// KEYS redis command
///
/// Returns all keys matching pattern.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///
///     // set keys
///     redis.exec(cmd::Set("firstname", "Jack")).await?;
///     redis.exec(cmd::Set("lastname", "Stuntman")).await?;
///
///     // get keys
///     let mut keys = redis.exec(cmd::Keys("*name*")).await?;
///     # keys.sort();
///
///     assert_eq!(&keys[..], &["firstname", "lastname"][..]);
///     Ok(())
/// }
/// ```
pub fn Keys<T>(key: T) -> KeysPatternCommand
where
    BulkString: From<T>,
{
    KeysPatternCommand(Request::Array(vec![
        Request::from_static("KEYS"),
        Request::BulkString(key.into()),
    ]))
}

pub struct KeysPatternCommand(Request);

impl Command for KeysPatternCommand {
    type Output = Vec<ByteString>;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val.try_into() {
            Ok(val) => Ok(val),
            Err((_, val)) => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}
