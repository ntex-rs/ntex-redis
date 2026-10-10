use super::{Command, CommandError, utils};
use crate::codec::{Arg, Frame, Writer};

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

        // EX|PX
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
