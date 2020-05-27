use super::{utils, Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// LINDEX redis command
///
/// Returns the element at index index in the list stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).collect::<String>()
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
pub fn LIndex<T>(key: T, index: i64) -> utils::BulkOutputCommand
where
    BulkString: From<T>,
{
    utils::BulkOutputCommand(Request::Array(vec![
        Request::from_static("LINDEX"),
        Request::BulkString(key.into()),
        Request::BulkInteger(index),
    ]))
}

/// LPOP redis command
///
/// Removes and returns the first element of the list stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create list with one value
///     redis.exec(cmd::LPush(&key, "value")).await?;
///
///     // pop first elements from the list
///     let value = redis.exec(cmd::LPop(&key)).await?;
///
///     assert_eq!(value.unwrap(), "value");
///     Ok(())
/// }
/// ```
pub fn LPop<T>(key: T) -> utils::BulkOutputCommand
where
    BulkString: From<T>,
{
    utils::BulkOutputCommand(Request::Array(vec![
        Request::from_static("LPOP"),
        Request::BulkString(key.into()),
    ]))
}

/// RPOP redis command
///
/// Removes and returns the last element of the list stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create list with one value
///     redis.exec(cmd::LPush(&key, "value")).await?;
///
///     // pop last elements from the list
///     let value = redis.exec(cmd::RPop(&key)).await?;
///
///     assert_eq!(value.unwrap(), "value");
///     Ok(())
/// }
/// ```
pub fn RPop<T>(key: T) -> utils::BulkOutputCommand
where
    BulkString: From<T>,
{
    utils::BulkOutputCommand(Request::Array(vec![
        Request::from_static("RPOP"),
        Request::BulkString(key.into()),
    ]))
}

/// LPUSH redis command
///
/// Insert all the specified values at the head of the list stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create list with one value
///     redis.exec(
///         cmd::LPush(&key, "value1")
///            .extend(vec!["value2", "value3", "value4"])
///     ).await?;
///
///     Ok(())
/// }
/// ```
///
/// `LPushCommand::if_exists()` method changes `LPUSH` command to `LPUSHX` command
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{thread_rng, Rng, distributions::Alphanumeric};
/// # fn gen_random_key() -> String {
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create list with one value only if key already exists
///     redis.exec(
///         cmd::LPush(&key, "value1")
///             .value("value2")
///             .if_exists()
///     ).await?;
///
///     Ok(())
/// }
/// ```
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

    /// Add a value to this command.
    pub fn value<T>(mut self, other: T) -> Self
    where
        BulkString: From<T>,
    {
        self.0.push(other.into());
        self
    }

    /// Add more values to this command.
    pub fn extend<T>(mut self, other: impl IntoIterator<Item = T>) -> Self
    where
        BulkString: From<T>,
    {
        self.0.extend(other.into_iter().map(|t| t.into()));
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
