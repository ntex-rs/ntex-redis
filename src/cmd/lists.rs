use super::{Command, CommandError, utils};
use crate::codec::{Arg, Frame, VarArgs, Writer};

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
pub fn LPush<T: Arg, V: Arg>(key: T, value: V) -> LPushCommand<T, V> {
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
pub fn RPush<T: Arg, V: Arg>(key: T, value: V) -> LPushCommand<T, V> {
    LPushCommand::new("RPUSH", key, value)
}

/// LPUSH and RPUSH command, see [`LPush`] and [`RPush`]
pub struct LPushCommand<A, B>(VarArgs<(&'static str, A, B)>);

impl<A, B> LPushCommand<A, B> {
    fn new(name: &'static str, key: A, value: B) -> Self {
        LPushCommand(VarArgs::with_head((name, key, value)))
    }

    /// Inserts specified values only if key already exists and holds a list.
    pub fn if_exists(mut self) -> Self {
        let cmd = match self.0.head_mut().0 {
            "LPUSH" | "LPUSHX" => "LPUSHX",
            _ => "RPUSHX",
        };
        self.0.head_mut().0 = cmd;
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

impl<A: Arg, B: Arg> Command for LPushCommand<A, B> {
    type Output = usize;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.int()? as usize)
    }
}

/// LLEN redis command
///
/// Returns the length of the list stored at `key`, `0` if the key does not exist.
pub fn LLen<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("LLEN", key))
}

/// LRANGE redis command
///
/// Returns the elements of the list stored at `key` between the inclusive
/// `start` and `stop` indexes. Negative indexes are counted from the end of
/// the list, `-1` is the last element.
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
///     redis.exec(cmd::RPush(&key, "a").extend(["b", "c"])).await?;
///
///     let items = redis.exec(cmd::LRange(&key, 0, -1)).await?;
///     assert_eq!(items, ["a", "b", "c"]);
///     assert_eq!(redis.exec(cmd::LLen(&key)).await?, 3);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn LRange<T: Arg>(
    key: T,
    start: i64,
    stop: i64,
) -> utils::ListOutputCommand<(&'static str, T, i64, i64)> {
    utils::ListOutputCommand(("LRANGE", key, start, stop))
}

/// LREM redis command
///
/// Removes the first `count` occurrences of `element` from the list stored
/// at `key`. Negative `count` removes elements moving from tail to head,
/// `0` removes all occurrences. Returns the number of removed elements.
pub fn LRem<T: Arg, V: Arg>(
    key: T,
    count: i64,
    element: V,
) -> utils::IntOutputCommand<(&'static str, T, i64, V)> {
    utils::IntOutputCommand(("LREM", key, count, element))
}

/// LSET redis command
///
/// Sets the list element at `index` to `element`. Returns an error if
/// the index is out of range.
pub fn LSet<T: Arg, V: Arg>(
    key: T,
    index: i64,
    element: V,
) -> utils::OkOutputCommand<(&'static str, T, i64, V)> {
    utils::OkOutputCommand(("LSET", key, index, element))
}

/// LTRIM redis command
///
/// Trims the list stored at `key` so that it contains only elements between
/// the inclusive `start` and `stop` indexes.
pub fn LTrim<T: Arg>(
    key: T,
    start: i64,
    stop: i64,
) -> utils::OkOutputCommand<(&'static str, T, i64, i64)> {
    utils::OkOutputCommand(("LTRIM", key, start, stop))
}

/// LINSERT redis command
///
/// Inserts `element` in the list stored at `key` before the `pivot` element,
/// use [`LInsertCommand::after`] to insert after it. Returns the length of
/// the list after the insert operation, `-1` if `pivot` was not found and
/// `0` if the key does not exist.
pub fn LInsert<T: Arg, P: Arg, V: Arg>(key: T, pivot: P, element: V) -> LInsertCommand<T, P, V> {
    LInsertCommand {
        key,
        pivot,
        element,
        after: false,
    }
}

/// LINSERT command, see [`LInsert`](crate::cmd::LInsert)
pub struct LInsertCommand<T, P, V> {
    key: T,
    pivot: P,
    element: V,
    after: bool,
}

impl<T, P, V> LInsertCommand<T, P, V> {
    /// Insert element after the pivot
    pub fn after(mut self) -> Self {
        self.after = true;
        self
    }
}

impl<T: Arg, P: Arg, V: Arg> Command for LInsertCommand<T, P, V> {
    type Output = i64;

    fn encode(self, w: &mut Writer<'_>) {
        let pos = if self.after { "AFTER" } else { "BEFORE" };
        w.args(("LINSERT", self.key, pos, self.pivot, self.element));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.int()
    }
}

/// LPOS redis command
///
/// Returns the index of the first matching `element` in the list stored at
/// `key`, `None` if there is no match.
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
///     redis.exec(cmd::RPush(&key, "a").extend(["b", "a"])).await?;
///
///     assert_eq!(redis.exec(cmd::LPos(&key, "a")).await?, Some(0));
///     assert_eq!(redis.exec(cmd::LPos(&key, "a").rank(2)).await?, Some(2));
///     assert_eq!(redis.exec(cmd::LPos(&key, "c")).await?, None);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn LPos<T: Arg, V: Arg>(key: T, element: V) -> LPosCommand<T, V> {
    LPosCommand {
        key,
        element,
        rank: None,
        max_len: None,
    }
}

/// LPOS command, see [`LPos`](crate::cmd::LPos)
pub struct LPosCommand<T, V> {
    key: T,
    element: V,
    rank: Option<i64>,
    max_len: Option<usize>,
}

impl<T, V> LPosCommand<T, V> {
    /// Return the `rank`-th match, negative rank searches from the tail
    pub fn rank(mut self, rank: i64) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Compare at most `len` elements, `0` means no limit
    pub fn max_len(mut self, len: usize) -> Self {
        self.max_len = Some(len);
        self
    }
}

impl<T: Arg, V: Arg> Command for LPosCommand<T, V> {
    type Output = Option<usize>;

    fn encode(self, w: &mut Writer<'_>) {
        w.array(3 + 2 * (usize::from(self.rank.is_some()) + usize::from(self.max_len.is_some())));
        w.arg("LPOS");
        w.arg(self.key);
        w.arg(self.element);
        if let Some(rank) = self.rank {
            w.arg("RANK");
            w.arg(rank);
        }
        if let Some(len) = self.max_len {
            w.arg("MAXLEN");
            w.arg(len);
        }
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        if frame.is_nil() {
            frame.bulk().map(|_| None)
        } else {
            Ok(Some(frame.int()? as usize))
        }
    }
}

/// List side, used by [`LMove`]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    /// Head of the list
    Left,
    /// Tail of the list
    Right,
}

impl Side {
    fn as_str(self) -> &'static str {
        match self {
            Side::Left => "LEFT",
            Side::Right => "RIGHT",
        }
    }
}

/// LMOVE redis command
///
/// Atomically removes the first or last element of the `source` list and
/// pushes it to the first or last position of the `destination` list.
/// Returns the moved element, `None` if `source` does not exist.
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
///     let src = gen_random_key();
///     let dst = gen_random_key();
///
///     redis.exec(cmd::RPush(&src, "a").extend(["b"])).await?;
///
///     let item = redis.exec(cmd::LMove(&src, &dst, cmd::Side::Right, cmd::Side::Left)).await?;
///     assert_eq!(item.unwrap(), "b");
///     assert_eq!(redis.exec(cmd::LRange(&dst, 0, -1)).await?, ["b"]);
///
///     redis.exec(cmd::Del(&src).key(&dst)).await?;
///     Ok(())
/// }
/// ```
pub fn LMove<S: Arg, D: Arg>(
    source: S,
    destination: D,
    from: Side,
    to: Side,
) -> utils::BulkOutputCommand<(&'static str, S, D, &'static str, &'static str)> {
    utils::BulkOutputCommand(("LMOVE", source, destination, from.as_str(), to.as_str()))
}
