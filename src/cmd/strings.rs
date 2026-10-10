use ntex_bytes::Bytes;

use super::{Command, CommandError, utils};
use crate::codec::{Arg, Frame, VarArgs, Writer};

/// GET redis command
///
/// Get the value of `key`. Returns `None` if the key does not exist.
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
///     // get value
///     let value = redis.exec(cmd::Get(&key)).await?;
///     assert_eq!(value.unwrap(), "value");
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn Get<T: Arg>(key: T) -> utils::BulkOutputCommand<(&'static str, T)> {
    utils::BulkOutputCommand(("GET", key))
}

/// SET redis command
///
/// Set key to hold the string value. Command returns `true` if value is set,
/// `false` if it is not set because of the `if_exists()`/`if_not_exists()` condition.
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
///     // set value with 10 seconds expiration
///     assert!(redis.exec(cmd::Set(&key, "value").expire_secs(10)).await?);
///
///     // key already exists, value is not set
///     assert!(!redis.exec(cmd::Set(&key, "other").if_not_exists()).await?);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn Set<T: Arg, V: Arg>(key: T, value: V) -> SetCommand<T, V> {
    SetCommand {
        key,
        value,
        expire: None,
        keepttl: false,
        exists: None,
    }
}

/// SET command, see [`Set`](crate::cmd::Set)
pub struct SetCommand<T, V> {
    key: T,
    value: V,
    expire: Option<(&'static str, i64)>,
    keepttl: bool,
    exists: Option<bool>,
}

impl<T, V> SetCommand<T, V> {
    /// Set the specified expire time, in seconds.
    pub fn expire_secs(mut self, secs: i64) -> Self {
        self.expire = Some(("EX", secs));
        self
    }

    /// Set the specified expire time, in milliseconds.
    pub fn expire_millis(mut self, millis: i64) -> Self {
        self.expire = Some(("PX", millis));
        self
    }

    /// Set the specified unix time at which the key will expire, in seconds.
    pub fn expire_at_secs(mut self, timestamp: i64) -> Self {
        self.expire = Some(("EXAT", timestamp));
        self
    }

    /// Set the specified unix time at which the key will expire, in milliseconds.
    pub fn expire_at_millis(mut self, timestamp: i64) -> Self {
        self.expire = Some(("PXAT", timestamp));
        self
    }

    /// Only set the key if it already exists.
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

impl<T: Arg, V: Arg> Command for SetCommand<T, V> {
    type Output = bool;

    fn encode(self, w: &mut Writer<'_>) {
        let len = 3
            + if self.expire.is_some() { 2 } else { 0 }
            + usize::from(self.exists.is_some())
            + usize::from(self.keepttl);

        w.array(len);
        w.arg("SET");
        w.arg(self.key);
        w.arg(self.value);

        // EX|PX|EXAT|PXAT
        if let Some((name, val)) = self.expire {
            w.arg(name);
            w.arg(val);
        }

        // NX|XX
        if let Some(exists) = self.exists {
            w.arg(if exists { "XX" } else { "NX" });
        }

        // KEEPTTL
        if self.keepttl {
            w.arg("KEEPTTL");
        }
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        if frame.is_nil() {
            Ok(false)
        } else if frame.peek()? == b'+' && frame.string()? == "OK" {
            Ok(true)
        } else {
            Err(frame.unexpected("Unexpected value"))
        }
    }
}

/// INCRBY redis command
///
/// Increments the number stored at `key` by `increment` and returns
/// the value after the increment. Missing key is set to `0` before the operation.
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
///     assert_eq!(redis.exec(cmd::IncrBy(&key, 5)).await?, 5);
///     assert_eq!(redis.exec(cmd::IncrBy(&key, -2)).await?, 3);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn IncrBy<T, I>(key: T, increment: I) -> utils::IntOutputCommand<(&'static str, T, i64)>
where
    T: Arg,
    i64: From<I>,
{
    utils::IntOutputCommand(("INCRBY", key, i64::from(increment)))
}

/// INCR redis command
///
/// Increments the number stored at `key` by one and returns the value
/// after the increment. Missing key is set to `0` before the operation.
pub fn Incr<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("INCR", key))
}

/// DECR redis command
///
/// Decrements the number stored at `key` by one and returns the value
/// after the decrement. Missing key is set to `0` before the operation.
pub fn Decr<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("DECR", key))
}

/// DECRBY redis command
///
/// Decrements the number stored at `key` by `decrement` and returns the value
/// after the decrement. Missing key is set to `0` before the operation.
pub fn DecrBy<T, I>(key: T, decrement: I) -> utils::IntOutputCommand<(&'static str, T, i64)>
where
    T: Arg,
    i64: From<I>,
{
    utils::IntOutputCommand(("DECRBY", key, i64::from(decrement)))
}

/// INCRBYFLOAT redis command
///
/// Increments the floating point number stored at `key` by `increment` and
/// returns the value after the increment. Missing key is set to `0` before
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
///     assert_eq!(redis.exec(cmd::IncrByFloat(&key, 1.5)).await?, 1.5);
///     assert_eq!(redis.exec(cmd::IncrByFloat(&key, -0.25)).await?, 1.25);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn IncrByFloat<T: Arg>(
    key: T,
    increment: f64,
) -> utils::FloatOutputCommand<(&'static str, T, f64)> {
    utils::FloatOutputCommand(("INCRBYFLOAT", key, increment))
}

/// APPEND redis command
///
/// Appends the `value` at the end of the string stored at `key`, missing key
/// is created. Returns the length of the string after the append operation.
pub fn Append<T: Arg, V: Arg>(key: T, value: V) -> utils::IntOutputCommand<(&'static str, T, V)> {
    utils::IntOutputCommand(("APPEND", key, value))
}

/// STRLEN redis command
///
/// Returns the length of the string value stored at `key`, `0` if the key
/// does not exist.
pub fn StrLen<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("STRLEN", key))
}

/// GETDEL redis command
///
/// Get the value of `key` and delete the key. Returns `None` if the key
/// does not exist.
pub fn GetDel<T: Arg>(key: T) -> utils::BulkOutputCommand<(&'static str, T)> {
    utils::BulkOutputCommand(("GETDEL", key))
}

/// GETRANGE redis command
///
/// Returns the substring of the string value stored at `key`, determined by
/// the inclusive `start` and `end` offsets. Negative offsets are counted from
/// the end of the string. Returns empty string if the key does not exist.
pub fn GetRange<T: Arg>(
    key: T,
    start: i64,
    end: i64,
) -> utils::BytesOutputCommand<(&'static str, T, i64, i64)> {
    utils::BytesOutputCommand(("GETRANGE", key, start, end))
}

/// SETRANGE redis command
///
/// Overwrites part of the string stored at `key`, starting at the specified
/// `offset`, for the entire length of `value`. Returns the length of
/// the string after it was modified.
pub fn SetRange<T: Arg, V: Arg>(
    key: T,
    offset: usize,
    value: V,
) -> utils::IntOutputCommand<(&'static str, T, usize, V)> {
    utils::IntOutputCommand(("SETRANGE", key, offset, value))
}

/// GETEX redis command
///
/// Get the value of `key` and optionally set its expiration. Returns `None`
/// if the key does not exist.
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
///
///     // get value and set 10 seconds expiration
///     let value = redis.exec(cmd::GetEx(&key).expire_secs(10)).await?;
///     assert_eq!(value.unwrap(), "value");
///     assert!(matches!(redis.exec(cmd::Ttl(&key)).await?, cmd::TtlResult::Seconds(1..=10)));
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn GetEx<T: Arg>(key: T) -> GetExCommand<T> {
    GetExCommand { key, expire: None }
}

/// GETEX command, see [`GetEx`](crate::cmd::GetEx)
pub struct GetExCommand<T> {
    key: T,
    expire: Option<(&'static str, Option<i64>)>,
}

impl<T> GetExCommand<T> {
    /// Set the specified expire time, in seconds.
    pub fn expire_secs(mut self, secs: i64) -> Self {
        self.expire = Some(("EX", Some(secs)));
        self
    }

    /// Set the specified expire time, in milliseconds.
    pub fn expire_millis(mut self, millis: i64) -> Self {
        self.expire = Some(("PX", Some(millis)));
        self
    }

    /// Set the specified unix time at which the key will expire, in seconds.
    pub fn expire_at_secs(mut self, timestamp: i64) -> Self {
        self.expire = Some(("EXAT", Some(timestamp)));
        self
    }

    /// Set the specified unix time at which the key will expire, in milliseconds.
    pub fn expire_at_millis(mut self, timestamp: i64) -> Self {
        self.expire = Some(("PXAT", Some(timestamp)));
        self
    }

    /// Remove the time to live associated with the key.
    pub fn persist(mut self) -> Self {
        self.expire = Some(("PERSIST", None));
        self
    }
}

impl<T: Arg> Command for GetExCommand<T> {
    type Output = Option<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        match self.expire {
            None => w.args(("GETEX", self.key)),
            Some((name, None)) => w.args(("GETEX", self.key, name)),
            Some((name, Some(val))) => w.args(("GETEX", self.key, name, val)),
        }
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.bulk()
    }
}

/// MGET redis command
///
/// Returns the values of all specified keys, `None` for keys that do not exist.
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
///     let key1 = gen_random_key();
///     let key2 = gen_random_key();
///
///     redis.exec(cmd::MSet(&key1, "value1").entry(&key2, "value2")).await?;
///
///     let values = redis.exec(cmd::MGet(&key1).key(&key2).key(gen_random_key())).await?;
///     assert_eq!(values[0].as_ref().unwrap(), "value1");
///     assert_eq!(values[1].as_ref().unwrap(), "value2");
///     assert_eq!(values[2], None);
///
///     redis.exec(cmd::Del(&key1).key(&key2)).await?;
///     Ok(())
/// }
/// ```
pub fn MGet<T: Arg>(key: T) -> MGetCommand<T> {
    MGetCommand(VarArgs::with_head(("MGET", key)))
}

/// MGET command, see [`MGet`](crate::cmd::MGet)
pub struct MGetCommand<A>(VarArgs<(&'static str, A)>);

impl<A> MGetCommand<A> {
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

impl<A: Arg> Command for MGetCommand<A> {
    type Output = Vec<Option<Bytes>>;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_opt_list(frame)
    }
}

/// MSET redis command
///
/// Sets the given keys to their respective values. Use [`MSetCommand::entry`]
/// to set multiple keys. Returns `true` if the keys were set, `false` if
/// no key was set because of the `if_not_exists()` condition.
///
/// See [`MGet`] for an example.
pub fn MSet<T: Arg, V: Arg>(key: T, value: V) -> MSetCommand<T, V> {
    MSetCommand(VarArgs::with_head(("MSET", key, value)))
}

/// MSET and MSETNX command, see [`MSet`](crate::cmd::MSet)
pub struct MSetCommand<A, B>(VarArgs<(&'static str, A, B)>);

impl<A, B> MSetCommand<A, B> {
    /// Add another key and value to set
    pub fn entry<K: Arg, V: Arg>(mut self, key: K, value: V) -> Self {
        self.add_entry(key, value);
        self
    }

    /// Add another key and value to set, by mutable reference
    pub fn add_entry<K: Arg, V: Arg>(&mut self, key: K, value: V) {
        self.0.push(key);
        self.0.push(value);
    }

    /// Set the keys only if none of them exist, changes `MSET` command to `MSETNX`
    pub fn if_not_exists(mut self) -> Self {
        self.0.head_mut().0 = "MSETNX";
        self
    }
}

impl<A: Arg, B: Arg> Command for MSetCommand<A, B> {
    type Output = bool;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        if frame.peek()? == b'+' {
            utils::read_ok(frame).map(|()| true)
        } else {
            utils::read_bool(frame)
        }
    }
}
