use std::convert::{TryFrom, TryInto};

use ntex_bytes::ByteString;

use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Response, Writer};

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

/// DEL and EXISTS command, see [`Del`](crate::cmd::Del) and [`Exists`](crate::cmd::Exists)
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

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Integer(val) => Ok(val as usize),
            _ => Err(CommandError::Output("Cannot parse response", val)),
        }
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

/// Result of the [`Ttl`] command
#[derive(Debug, PartialEq, Eq)]
pub enum TtlResult {
    /// Remaining time to live, in seconds
    Seconds(i64),
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

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val.try_into() {
            Ok(val) => Ok(val),
            Err((_, val)) => Err(CommandError::Output("Cannot parse response", val)),
        }
    }
}
