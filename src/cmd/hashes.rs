use std::convert::TryFrom;

use ntex_bytes::Bytes;
use ntex_util::HashMap;

use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Response, Writer};

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
pub fn HGet<T: Arg, V: Arg>(key: T, field: V) -> utils::BulkOutputCommand<(&'static str, T, V)> {
    utils::BulkOutputCommand(("HGET", key, field))
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
pub fn HGetAll<T: Arg>(key: T) -> HGetAllCommand<T> {
    HGetAllCommand(key)
}

/// HGETALL command, see [`HGetAll`](crate::cmd::HGetAll)
pub struct HGetAllCommand<T>(T);

impl<T: Arg> Command for HGetAllCommand<T> {
    type Output = HashMap<Bytes, Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(("HGETALL", self.0));
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
pub fn HSet<T: Arg, K: Arg, V: Arg>(key: T, field: K, value: V) -> HSetCommand {
    let mut args = ArgList::new("HSET");
    args.push(key);
    args.push(field);
    args.push(value);
    HSetCommand(args)
}

/// HSET command, see [`HSet`](crate::cmd::HSet)
pub struct HSetCommand(ArgList);

impl HSetCommand {
    /// Add another field and value to set
    pub fn entry<K: Arg, V: Arg>(mut self, field: K, value: V) -> Self {
        self.add_entry(field, value);
        self
    }

    /// Add another field and value to set, by mutable reference
    pub fn add_entry<K: Arg, V: Arg>(&mut self, field: K, value: V) {
        self.0.push(field);
        self.0.push(value);
    }
}

impl Command for HSetCommand {
    type Output = usize;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
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
pub fn HDel<T: Arg, K: Arg>(key: T, field: K) -> HDelCommand {
    let mut args = ArgList::new("HDEL");
    args.push(key);
    args.push(field);
    HDelCommand(args)
}

/// HDEL command, see [`HDel`](crate::cmd::HDel)
pub struct HDelCommand(ArgList);

impl HDelCommand {
    /// Add another field to remove
    pub fn remove<K: Arg>(mut self, field: K) -> Self {
        self.0.push(field);
        self
    }

    /// Add more fields to remove
    pub fn remove_all<T: Arg>(mut self, other: impl IntoIterator<Item = T>) -> Self {
        for field in other {
            self.0.push(field);
        }
        self
    }
}

impl Command for HDelCommand {
    type Output = usize;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
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
pub fn HLen<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("HLEN", key))
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
pub fn HIncrBy<T, K, I>(
    key: T,
    field: K,
    increment: I,
) -> utils::IntOutputCommand<(&'static str, T, K, i64)>
where
    T: Arg,
    K: Arg,
    i64: From<I>,
{
    utils::IntOutputCommand(("HINCRBY", key, field, i64::from(increment)))
}
