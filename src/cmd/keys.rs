use super::{Command, CommandError};
use crate::codec::{BulkString, Request, Response};

/// DEL redis command
///
/// Removes the specified keys. A key is ignored if it does not exist.
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
///     // set string value
///     redis.exec(cmd::Set(&key, "value"));
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
/// #    thread_rng().sample_iter(&Alphanumeric).take(12).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // set string value
///     redis.exec(cmd::Set(&key, "value"));
///
///     // check keys existence
///     let value = redis.exec(
///         cmd::Exists(&key)
///             .keys(vec!["test_1", "test_2"])
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
