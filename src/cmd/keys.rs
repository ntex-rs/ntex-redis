use ntex_bytes::{ByteString, Bytes};

use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Frame, Writer};

/// DEL redis command
///
/// Removes the specified keys. A key is ignored if it does not exist.
/// Returns the number of keys that were removed.
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
///     let missing = gen_random_key();
///
///     // set string value
///     redis.exec(cmd::Set(&key, "value")).await?;
///
///     // remove keys
///     let value = redis.exec(cmd::Del(&key).key(&missing)).await?;
///
///     assert_eq!(value, 1);
///     Ok(())
/// }
/// ```
pub fn Del<T: Arg>(key: T) -> KeysCommand {
    KeysCommand::new("DEL", key)
}

/// EXISTS redis command
///
/// Returns the number of specified keys that exist.
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
///     // set string value
///     redis.exec(cmd::Set(&key, "value")).await?;
///
///     // check keys existence
///     let value = redis.exec(
///         cmd::Exists(&key).keys(vec![gen_random_key(), gen_random_key()])
///     ).await?;
///
///     assert_eq!(value, 1);
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn Exists<T: Arg>(key: T) -> KeysCommand {
    KeysCommand::new("EXISTS", key)
}

/// UNLINK redis command
///
/// Removes the specified keys like [`Del`], memory is reclaimed
/// asynchronously. Returns the number of keys that were unlinked.
pub fn Unlink<T: Arg>(key: T) -> KeysCommand {
    KeysCommand::new("UNLINK", key)
}

/// TOUCH redis command
///
/// Alters the last access time of the keys. Returns the number of keys
/// that were touched.
pub fn Touch<T: Arg>(key: T) -> KeysCommand {
    KeysCommand::new("TOUCH", key)
}

/// DEL, EXISTS, UNLINK and TOUCH command, see [`Del`](crate::cmd::Del),
/// [`Exists`](crate::cmd::Exists), [`Unlink`](crate::cmd::Unlink)
/// and [`Touch`](crate::cmd::Touch)
pub struct KeysCommand(ArgList);

impl KeysCommand {
    fn new<T: Arg>(name: &'static str, key: T) -> Self {
        let mut args = ArgList::new(name);
        args.push(key);
        KeysCommand(args)
    }

    /// Add a key to this command.
    pub fn key<T: Arg>(mut self, other: T) -> Self {
        self.0.push(other);
        self
    }

    /// Add more keys to this command.
    pub fn keys<T: Arg>(mut self, other: impl IntoIterator<Item = T>) -> Self {
        for key in other {
            self.0.push(key);
        }
        self
    }
}

impl Command for KeysCommand {
    type Output = usize;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.int()? as usize)
    }
}

/// EXPIRE redis command
///
/// Set a timeout on `key`, in seconds. After the timeout has expired, the key
/// will automatically be deleted. Returns `true` if the timeout was set,
/// `false` if the key does not exist.
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
///     redis.exec(cmd::Set(&key, "value")).await?;
///     assert!(redis.exec(cmd::Expire(&key, 10)).await?);
///
///     let ttl = redis.exec(cmd::Ttl(&key)).await?;
///     assert!(matches!(ttl, cmd::TtlResult::Seconds(1..=10)));
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn Expire<T, S>(key: T, seconds: S) -> utils::BoolOutputCommand<(&'static str, T, i64)>
where
    T: Arg,
    i64: From<S>,
{
    utils::BoolOutputCommand(("EXPIRE", key, i64::from(seconds)))
}

/// EXPIREAT redis command
///
/// Set `key` to expire at the specified absolute unix timestamp, in seconds.
/// A timestamp in the past deletes the key immediately. Returns `true` if
/// the timeout was set, `false` if the key does not exist.
pub fn ExpireAt<T, S>(key: T, timestamp: S) -> utils::BoolOutputCommand<(&'static str, T, i64)>
where
    T: Arg,
    i64: From<S>,
{
    utils::BoolOutputCommand(("EXPIREAT", key, i64::from(timestamp)))
}

/// PEXPIRE redis command
///
/// Set a timeout on `key`, in milliseconds. Returns `true` if the timeout
/// was set, `false` if the key does not exist.
pub fn PExpire<T, S>(key: T, millis: S) -> utils::BoolOutputCommand<(&'static str, T, i64)>
where
    T: Arg,
    i64: From<S>,
{
    utils::BoolOutputCommand(("PEXPIRE", key, i64::from(millis)))
}

/// PEXPIREAT redis command
///
/// Set `key` to expire at the specified absolute unix timestamp, in
/// milliseconds. Returns `true` if the timeout was set, `false` if the key
/// does not exist.
pub fn PExpireAt<T, S>(key: T, timestamp: S) -> utils::BoolOutputCommand<(&'static str, T, i64)>
where
    T: Arg,
    i64: From<S>,
{
    utils::BoolOutputCommand(("PEXPIREAT", key, i64::from(timestamp)))
}

/// PERSIST redis command
///
/// Remove the existing timeout on `key`. Returns `true` if the timeout was
/// removed, `false` if the key does not exist or has no timeout.
pub fn Persist<T: Arg>(key: T) -> utils::BoolOutputCommand<(&'static str, T)> {
    utils::BoolOutputCommand(("PERSIST", key))
}

/// TTL redis command
///
/// Returns the remaining time to live of a `key` that has a timeout.
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
///     redis.exec(cmd::Set(&key, "value")).await?;
///     assert_eq!(redis.exec(cmd::Ttl(&key)).await?, cmd::TtlResult::NoExpire);
///
///     redis.exec(cmd::Del(&key)).await?;
///     assert_eq!(redis.exec(cmd::Ttl(&key)).await?, cmd::TtlResult::NotFound);
///     Ok(())
/// }
/// ```
pub fn Ttl<T: Arg>(key: T) -> TtlCommand<T> {
    TtlCommand(key)
}

/// Result of the [`Ttl`] and [`PTtl`] commands
#[derive(Debug, PartialEq, Eq)]
pub enum TtlResult {
    /// Remaining time to live, in seconds
    Seconds(i64),
    /// Remaining time to live, in milliseconds, returned by [`PTtl`]
    Millis(i64),
    /// Key exists but has no associated expire
    NoExpire,
    /// Key does not exist
    NotFound,
}

/// TTL command, see [`Ttl`]
pub struct TtlCommand<T>(T);

impl<T: Arg> Command for TtlCommand<T> {
    type Output = TtlResult;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(("TTL", self.0));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(match frame.int()? {
            -1 => TtlResult::NoExpire,
            -2 => TtlResult::NotFound,
            s => TtlResult::Seconds(s),
        })
    }
}

/// PTTL redis command
///
/// Returns the remaining time to live of a `key` that has a timeout,
/// in milliseconds.
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
///     redis.exec(cmd::Set(&key, "value").expire_millis(5000)).await?;
///     let ttl = redis.exec(cmd::PTtl(&key)).await?;
///     assert!(matches!(ttl, cmd::TtlResult::Millis(1..=5000)));
///
///     assert!(redis.exec(cmd::Persist(&key)).await?);
///     assert_eq!(redis.exec(cmd::PTtl(&key)).await?, cmd::TtlResult::NoExpire);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn PTtl<T: Arg>(key: T) -> PTtlCommand<T> {
    PTtlCommand(key)
}

/// PTTL command, see [`PTtl`]
pub struct PTtlCommand<T>(T);

impl<T: Arg> Command for PTtlCommand<T> {
    type Output = TtlResult;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(("PTTL", self.0));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(match frame.int()? {
            -1 => TtlResult::NoExpire,
            -2 => TtlResult::NotFound,
            s => TtlResult::Millis(s),
        })
    }
}

/// TYPE redis command
///
/// Returns the type of the value stored at `key`, i.e. `string`, `list`,
/// `set`, `zset`, `hash` or `stream`. Returns `none` if the key does not exist.
pub fn Type<T: Arg>(key: T) -> utils::StringOutputCommand<(&'static str, T)> {
    utils::StringOutputCommand(("TYPE", key))
}

/// RENAME redis command
///
/// Renames `key` to `new_key`, existing `new_key` is overwritten.
/// Returns an error if `key` does not exist.
pub fn Rename<T: Arg, N: Arg>(key: T, new_key: N) -> utils::OkOutputCommand<(&'static str, T, N)> {
    utils::OkOutputCommand(("RENAME", key, new_key))
}

/// RENAMENX redis command
///
/// Renames `key` to `new_key` if `new_key` does not exist. Returns `true`
/// if the key was renamed, `false` if `new_key` already exists.
pub fn RenameNx<T: Arg, N: Arg>(
    key: T,
    new_key: N,
) -> utils::BoolOutputCommand<(&'static str, T, N)> {
    utils::BoolOutputCommand(("RENAMENX", key, new_key))
}

/// SCAN redis command
///
/// Incrementally iterates over the keys of the database. Start iteration with
/// cursor `0`, then pass the returned cursor to the next call. Iteration is
/// complete when returned cursor is `0`. A key may be returned multiple times.
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
///     let prefix = gen_random_key();
///     let key = format!("{prefix}:key");
///     redis.exec(cmd::Set(&key, "value")).await?;
///
///     let mut keys = Vec::new();
///     let mut cursor = 0;
///     loop {
///         let (next, batch) = redis
///             .exec(cmd::Scan(cursor).pattern(format!("{prefix}:*")).count(100))
///             .await?;
///         keys.extend(batch);
///         if next == 0 {
///             break;
///         }
///         cursor = next;
///     }
///     assert!(keys.iter().any(|k| k == key.as_bytes()));
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn Scan(cursor: u64) -> utils::ScanCommand<Bytes> {
    let mut args = ArgList::new("SCAN");
    args.push(cursor);
    utils::ScanCommand::new(args)
}

/// KEYS redis command
///
/// Returns all keys matching pattern.
///
/// This command may block the server for a long time on large databases,
/// use with care in production.
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
///     let prefix = gen_random_key();
///     let first = format!("{prefix}:firstname");
///     let last = format!("{prefix}:lastname");
///
///     // set keys
///     redis.exec(cmd::Set(&first, "Jack")).await?;
///     redis.exec(cmd::Set(&last, "Stuntman")).await?;
///
///     // get keys
///     let mut keys = redis.exec(cmd::Keys(format!("{prefix}:*name"))).await?;
///     # keys.sort();
///
///     assert_eq!(keys, [first.as_str(), last.as_str()]);
///
///     redis.exec(cmd::Del(&first).key(&last)).await?;
///     Ok(())
/// }
/// ```
pub fn Keys<T: Arg>(pattern: T) -> KeysPatternCommand<T> {
    KeysPatternCommand(pattern)
}

/// KEYS command, see [`Keys`]
pub struct KeysPatternCommand<T>(T);

impl<T: Arg> Command for KeysPatternCommand<T> {
    type Output = Vec<ByteString>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(("KEYS", self.0));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        let Some(len) = frame.array()? else {
            return Err(frame.unexpected("Cannot be converted into a vector"));
        };
        let mut keys = Vec::with_capacity(len);
        for _ in 0..len {
            keys.push(frame.string()?);
        }
        Ok(keys)
    }
}
