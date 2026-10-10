use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Response, Writer};

/// LINDEX redis command
///
/// Returns the element at index index in the list stored at key.
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
///     // create list with one value
///     redis.exec(cmd::LPush(&key, "value")).await?;
///
///     // get value by index
///     let value = redis.exec(cmd::LIndex(&key, 0)).await?;
///
///     assert_eq!(value.unwrap(), "value");
///     Ok(())
/// }
/// ```
pub fn LIndex<T: Arg>(key: T, index: i64) -> utils::BulkOutputCommand<(&'static str, T, i64)> {
    utils::BulkOutputCommand(("LINDEX", key, index))
}

/// LPOP redis command
///
/// Removes and returns the first element of the list stored at key.
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
pub fn LPop<T: Arg>(key: T) -> utils::BulkOutputCommand<(&'static str, T)> {
    utils::BulkOutputCommand(("LPOP", key))
}

/// RPOP redis command
///
/// Removes and returns the last element of the list stored at key.
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
pub fn RPop<T: Arg>(key: T) -> utils::BulkOutputCommand<(&'static str, T)> {
    utils::BulkOutputCommand(("RPOP", key))
}

/// LPUSH redis command
///
/// Insert all the specified values at the head of the list stored at key.
/// Returns the length of the list after the push operation.
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
pub fn LPush<T: Arg, V: Arg>(key: T, value: V) -> LPushCommand {
    LPushCommand::new("LPUSH", key, value)
}

/// RPUSH redis command
///
/// Insert all the specified values at the tail of the list stored at key.
/// Returns the length of the list after the push operation.
///
/// See [`LPush`] for additional options.
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
///     redis.exec(cmd::RPush(&key, "value1").value("value2")).await?;
///
///     // get last value
///     let value = redis.exec(cmd::LIndex(&key, -1)).await?;
///     assert_eq!(value.unwrap(), "value2");
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn RPush<T: Arg, V: Arg>(key: T, value: V) -> LPushCommand {
    LPushCommand::new("RPUSH", key, value)
}

/// LPUSH and RPUSH command, see [`LPush`] and [`RPush`]
pub struct LPushCommand(ArgList);

impl LPushCommand {
    fn new<T: Arg, V: Arg>(name: &'static str, key: T, value: V) -> Self {
        let mut args = ArgList::new(name);
        args.push(key);
        args.push(value);
        LPushCommand(args)
    }

    /// Inserts specified values only if key already exists and holds a list.
    pub fn if_exists(mut self) -> Self {
        let cmd = match self.0.name() {
            "LPUSH" | "LPUSHX" => "LPUSHX",
            _ => "RPUSHX",
        };
        self.0.set_name(cmd);
        self
    }

    /// Add a value to this command.
    pub fn value<T: Arg>(mut self, other: T) -> Self {
        self.0.push(other);
        self
    }

    /// Add more values to this command.
    pub fn extend<T: Arg>(mut self, other: impl IntoIterator<Item = T>) -> Self {
        for val in other {
            self.0.push(val);
        }
        self
    }
}

impl Command for LPushCommand {
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
