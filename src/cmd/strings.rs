use super::{Command, CommandError, utils};
use crate::codec::{BulkString, Request, Response};

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
pub fn Get<T>(key: T) -> utils::BulkOutputCommand
where
    BulkString: From<T>,
{
    utils::BulkOutputCommand(Request::Array(vec![
        Request::from_static("GET"),
        Request::BulkString(key.into()),
    ]))
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
pub fn Set<T, V>(key: T, value: V) -> SetCommand
where
    BulkString: From<T> + From<V>,
{
    SetCommand {
        req: vec![
            Request::from_bstatic(b"SET"),
            Request::BulkString(key.into()),
            Request::BulkString(value.into()),
        ],
        expire: Expire::None,
        keepttl: false,
        exists: None,
    }
}

enum Expire {
    None,
    Ex(Request),
    Px(Request),
}

/// SET command, see [`Set`](crate::cmd::Set)
pub struct SetCommand {
    req: Vec<Request>,
    expire: Expire,
    keepttl: bool,
    exists: Option<bool>,
}

impl SetCommand {
    /// Set the specified expire time, in seconds.
    pub fn expire_secs(mut self, secs: i64) -> Self {
        self.expire = Expire::Ex(Request::BulkInteger(secs));
        self
    }

    /// Set the specified expire time, in milliseconds.
    pub fn expire_millis(mut self, millis: i64) -> Self {
        self.expire = Expire::Px(Request::BulkInteger(millis));
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

impl Command for SetCommand {
    type Output = bool;

    fn to_request(mut self) -> Request {
        // EX|PX
        match self.expire {
            Expire::None => (),
            Expire::Ex(r) => {
                self.req.push(Request::from_bstatic(b"EX"));
                self.req.push(r);
            }
            Expire::Px(r) => {
                self.req.push(Request::from_bstatic(b"PX"));
                self.req.push(r);
            }
        }

        // NX|XX
        if let Some(exists) = self.exists {
            self.req.push(if exists {
                Request::from_bstatic(b"XX")
            } else {
                Request::from_bstatic(b"NX")
            });
        }

        // KEEPTTL
        if self.keepttl {
            self.req.push(Request::from_bstatic(b"KEEPTTL"))
        }

        Request::Array(self.req)
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::Nil => Ok(false),
            Response::String(string) => match string.as_ref() {
                "OK" => Ok(true),
                _ => Err(CommandError::Output(
                    "Unexpected value within String",
                    Response::String(string),
                )),
            },
            _ => Err(CommandError::Output("Unexpected value", val)),
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
pub fn IncrBy<T, I>(key: T, increment: I) -> utils::IntOutputCommand
where
    BulkString: From<T>,
    i64: From<I>,
{
    utils::IntOutputCommand(Request::Array(vec![
        Request::from_static("INCRBY"),
        Request::BulkString(key.into()),
        Request::BulkString(i64::from(increment).to_string().into()),
    ]))
}
