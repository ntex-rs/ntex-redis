use std::convert::TryFrom;

use ntex_bytes::Bytes;
use ntex_util::HashMap;

use super::{Command, CommandError, utils};
use crate::codec::{BulkString, Request, Response};

/// HGET redis command
///
/// Returns the value associated with field in the hash stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{Rng, distr::Alphanumeric, rng};
/// # fn gen_random_key() -> String {
/// #    rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create hashmap
///     redis.exec(cmd::HSet(&key, "test-key", "value")).await?;
///
///     // get field value
///     let value = redis.exec(cmd::HGet(&key, "test-key")).await?;
///
///     assert_eq!(value.unwrap(), "value");
///     Ok(())
/// }
/// ```
pub fn HGet<T, V>(key: T, field: V) -> utils::BulkOutputCommand
where
    BulkString: From<T> + From<V>,
{
    utils::BulkOutputCommand(Request::Array(vec![
        Request::from_static("HGET"),
        Request::BulkString(key.into()),
        Request::BulkString(field.into()),
    ]))
}

/// HGETALL redis command
///
/// Returns all fields and values of the hash stored at `key`.
/// Returns an empty map if the key does not exist.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{Rng, distr::Alphanumeric, rng};
/// # fn gen_random_key() -> String {
/// #    rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     redis.exec(cmd::HSet(&key, "field1", "value1").entry("field2", "value2")).await?;
///
///     let map = redis.exec(cmd::HGetAll(&key)).await?;
///     assert_eq!(map.len(), 2);
///     assert_eq!(map.get(&b"field1"[..]).unwrap(), "value1");
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn HGetAll<T>(key: T) -> HGetAllCommand
where
    BulkString: From<T>,
{
    HGetAllCommand(vec![
        Request::from_static("HGETALL"),
        Request::BulkString(key.into()),
    ])
}

/// HGETALL command, see [`HGetAll`](crate::cmd::HGetAll)
pub struct HGetAllCommand(Vec<Request>);

impl Command for HGetAllCommand {
    type Output = HashMap<Bytes, Bytes>;

    fn to_request(self) -> Request {
        Request::Array(self.0)
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        Ok(HashMap::try_from(val)?)
    }
}

/// HSET redis command
///
/// Sets field in the hash stored at key to value. Returns the number of
/// fields that were added. Use [`HSetCommand::entry`] to set multiple fields.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{Rng, distr::Alphanumeric, rng};
/// # fn gen_random_key() -> String {
/// #    rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create hashmap and set field
///     redis.exec(cmd::HSet(&key, "test-key", "value")).await?;
///
///     // get field value
///     let value = redis.exec(cmd::HGet(&key, "test-key")).await?;
///
///     assert_eq!(value.unwrap(), "value");
///     Ok(())
/// }
/// ```
pub fn HSet<T, K, V>(key: T, field: K, value: V) -> HSetCommand
where
    BulkString: From<T> + From<K> + From<V>,
{
    HSetCommand(vec![
        Request::from_static("HSET"),
        Request::BulkString(key.into()),
        Request::BulkString(field.into()),
        Request::BulkString(value.into()),
    ])
}

/// HSET command, see [`HSet`](crate::cmd::HSet)
pub struct HSetCommand(Vec<Request>);

impl HSetCommand {
    /// Add another field and value to set
    pub fn entry<K, V>(mut self, field: K, value: V) -> Self
    where
        BulkString: From<K> + From<V>,
    {
        self.0.push(field.into());
        self.0.push(value.into());
        self
    }

    /// Add another field and value to set, by mutable reference
    pub fn add_entry<K, V>(&mut self, field: K, value: V)
    where
        BulkString: From<K> + From<V>,
    {
        self.0.push(field.into());
        self.0.push(value.into());
    }
}

impl Command for HSetCommand {
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

/// HDEL redis command
///
/// Removes the specified fields from the hash stored at key. Returns
/// the number of fields that were removed.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{Rng, distr::Alphanumeric, rng};
/// # fn gen_random_key() -> String {
/// #    rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create hashmap and set field
///     redis.exec(cmd::HSet(&key, "test-key", "value")).await?;
///
///     // delete hashmap field
///     let value = redis.exec(cmd::HDel(&key, "test-key")).await?;
///
///     assert_eq!(value, 1);
///     Ok(())
/// }
/// ```
pub fn HDel<T, K>(key: T, field: K) -> HDelCommand
where
    BulkString: From<T> + From<K>,
{
    HDelCommand(vec![
        Request::from_static("HDEL"),
        Request::BulkString(key.into()),
        Request::BulkString(field.into()),
    ])
}

/// HDEL command, see [`HDel`](crate::cmd::HDel)
pub struct HDelCommand(Vec<Request>);

impl HDelCommand {
    /// Add another field to remove
    pub fn remove<K>(mut self, field: K) -> Self
    where
        BulkString: From<K>,
    {
        self.0.push(field.into());
        self
    }

    /// Add more fields to remove
    pub fn remove_all<T>(mut self, other: impl IntoIterator<Item = T>) -> Self
    where
        BulkString: From<T>,
    {
        self.0.extend(other.into_iter().map(|t| t.into()));
        self
    }
}

impl Command for HDelCommand {
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

/// HLEN redis command
///
/// Returns the number of fields contained in the hash stored at key.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{Rng, distr::Alphanumeric, rng};
/// # fn gen_random_key() -> String {
/// #    rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     // create hashmap and set field
///     redis.exec(cmd::HSet(&key, "test-key", "value")).await?;
///
///     // get len of hashmap
///     let value = redis.exec(cmd::HLen(&key)).await?;
///
///     assert_eq!(value, 1);
///     Ok(())
/// }
/// ```
pub fn HLen<T>(key: T) -> utils::IntOutputCommand
where
    BulkString: From<T>,
{
    utils::IntOutputCommand(Request::Array(vec![
        Request::from_static("HLEN"),
        Request::BulkString(key.into()),
    ]))
}

/// HINCRBY redis command
///
/// Increments the number stored at `field` in the hash stored at `key` by `increment`
/// and returns the value after the increment. Missing field is set to `0` before
/// the operation.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
/// # use rand::{Rng, distr::Alphanumeric, rng};
/// # fn gen_random_key() -> String {
/// #    rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect::<String>()
/// # }
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///     let key = gen_random_key();
///
///     assert_eq!(redis.exec(cmd::HIncrBy(&key, "field", 5)).await?, 5);
///     assert_eq!(redis.exec(cmd::HIncrBy(&key, "field", -2)).await?, 3);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn HIncrBy<T, K, I>(key: T, field: K, increment: I) -> utils::IntOutputCommand
where
    BulkString: From<T> + From<K>,
    i64: From<I>,
{
    utils::IntOutputCommand(Request::Array(vec![
        Request::from_static("HINCRBY"),
        Request::BulkString(key.into()),
        Request::BulkString(field.into()),
        Request::BulkString(i64::from(increment).to_string().into()),
    ]))
}
