use ntex_bytes::Bytes;
use ntex_util::HashMap;

use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Frame, Writer};

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

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        let Some(len) = frame.array()? else {
            return Err(frame.unexpected("Cannot be converted into a hashmap"));
        };
        if len % 2 != 0 {
            return Err(frame.unexpected("Cannot convert an odd number of elements into a hashmap"));
        }
        let mut map = HashMap::with_capacity_and_hasher(len / 2, Default::default());
        for _ in 0..len / 2 {
            let key = frame.bytes()?;
            map.insert(key, frame.bytes()?);
        }
        Ok(map)
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

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.int()? as usize)
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

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.int()? as usize)
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

/// HINCRBYFLOAT redis command
///
/// Increments the floating point number stored at `field` in the hash stored
/// at `key` by `increment` and returns the value after the increment.
pub fn HIncrByFloat<T: Arg, K: Arg>(
    key: T,
    field: K,
    increment: f64,
) -> utils::FloatOutputCommand<(&'static str, T, K, f64)> {
    utils::FloatOutputCommand(("HINCRBYFLOAT", key, field, increment))
}

/// HMGET redis command
///
/// Returns the values of the specified fields in the hash stored at `key`,
/// `None` for fields that do not exist.
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
///     redis.exec(cmd::HSet(&key, "a", "1").entry("b", "2")).await?;
///
///     let values = redis.exec(cmd::HMGet(&key, "a").field("c").field("b")).await?;
///     assert_eq!(values[0].as_ref().unwrap(), "1");
///     assert_eq!(values[1], None);
///     assert_eq!(values[2].as_ref().unwrap(), "2");
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn HMGet<T: Arg, K: Arg>(key: T, field: K) -> HMGetCommand {
    let mut args = ArgList::new("HMGET");
    args.push(key);
    args.push(field);
    HMGetCommand(args)
}

/// HMGET command, see [`HMGet`](crate::cmd::HMGet)
pub struct HMGetCommand(ArgList);

impl HMGetCommand {
    /// Add a field to this command.
    pub fn field<K: Arg>(mut self, field: K) -> Self {
        self.0.push(field);
        self
    }

    /// Add more fields to this command.
    pub fn fields<K: Arg>(mut self, other: impl IntoIterator<Item = K>) -> Self {
        for field in other {
            self.0.push(field);
        }
        self
    }
}

impl Command for HMGetCommand {
    type Output = Vec<Option<Bytes>>;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_opt_list(frame)
    }
}

/// HEXISTS redis command
///
/// Returns `true` if `field` exists in the hash stored at `key`.
pub fn HExists<T: Arg, K: Arg>(key: T, field: K) -> utils::BoolOutputCommand<(&'static str, T, K)> {
    utils::BoolOutputCommand(("HEXISTS", key, field))
}

/// HSETNX redis command
///
/// Sets `field` in the hash stored at `key` to `value`, only if `field` does
/// not exist. Returns `true` if the field was set.
pub fn HSetNx<T: Arg, K: Arg, V: Arg>(
    key: T,
    field: K,
    value: V,
) -> utils::BoolOutputCommand<(&'static str, T, K, V)> {
    utils::BoolOutputCommand(("HSETNX", key, field, value))
}

/// HKEYS redis command
///
/// Returns all field names in the hash stored at `key`.
pub fn HKeys<T: Arg>(key: T) -> utils::ListOutputCommand<(&'static str, T)> {
    utils::ListOutputCommand(("HKEYS", key))
}

/// HVALS redis command
///
/// Returns all values in the hash stored at `key`.
pub fn HVals<T: Arg>(key: T) -> utils::ListOutputCommand<(&'static str, T)> {
    utils::ListOutputCommand(("HVALS", key))
}

/// HSTRLEN redis command
///
/// Returns the length of the value of `field` in the hash stored at `key`,
/// `0` if the key or the field do not exist.
pub fn HStrLen<T: Arg, K: Arg>(key: T, field: K) -> utils::IntOutputCommand<(&'static str, T, K)> {
    utils::IntOutputCommand(("HSTRLEN", key, field))
}

/// HSCAN redis command
///
/// Incrementally iterates over fields and values of the hash stored at `key`.
/// See [`Scan`](crate::cmd::Scan) for the iteration semantics.
pub fn HScan<T: Arg>(key: T, cursor: u64) -> utils::ScanCommand<(Bytes, Bytes)> {
    let mut args = ArgList::new("HSCAN");
    args.push(key);
    args.push(cursor);
    utils::ScanCommand::new(args)
}
