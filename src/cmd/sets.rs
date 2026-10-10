use ntex_bytes::Bytes;

use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Frame, Writer};

/// SADD redis command
///
/// Adds the members to the set stored at `key`. Returns the number of members
/// that were added, not including members already present in the set.
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
///     assert_eq!(redis.exec(cmd::SAdd(&key, "a").members(["b", "c"])).await?, 3);
///     assert_eq!(redis.exec(cmd::SRem(&key, "a").member("x")).await?, 1);
///
///     let mut members = redis.exec(cmd::SMembers(&key)).await?;
///     members.sort();
///     assert_eq!(members, ["b", "c"]);
///     assert!(redis.exec(cmd::SIsMember(&key, "b")).await?);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn SAdd<T: Arg, M: Arg>(key: T, member: M) -> MembersCommand {
    MembersCommand::new("SADD", key, member)
}

/// SREM redis command
///
/// Removes the members from the set stored at `key`. Returns the number of
/// members that were removed. See [`SAdd`] for an example.
pub fn SRem<T: Arg, M: Arg>(key: T, member: M) -> MembersCommand {
    MembersCommand::new("SREM", key, member)
}

/// SADD, SREM and ZREM command, see [`SAdd`](crate::cmd::SAdd),
/// [`SRem`](crate::cmd::SRem) and [`ZRem`](crate::cmd::ZRem)
pub struct MembersCommand(ArgList);

impl MembersCommand {
    pub(crate) fn new<T: Arg, M: Arg>(name: &'static str, key: T, member: M) -> Self {
        let mut args = ArgList::new(name);
        args.push(key);
        args.push(member);
        MembersCommand(args)
    }

    /// Add a member to this command.
    pub fn member<M: Arg>(mut self, member: M) -> Self {
        self.0.push(member);
        self
    }

    /// Add more members to this command.
    pub fn members<M: Arg>(mut self, other: impl IntoIterator<Item = M>) -> Self {
        for member in other {
            self.0.push(member);
        }
        self
    }
}

impl Command for MembersCommand {
    type Output = usize;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.int()? as usize)
    }
}

/// SMEMBERS redis command
///
/// Returns all members of the set stored at `key`.
pub fn SMembers<T: Arg>(key: T) -> utils::ListOutputCommand<(&'static str, T)> {
    utils::ListOutputCommand(("SMEMBERS", key))
}

/// SISMEMBER redis command
///
/// Returns `true` if `member` is a member of the set stored at `key`.
pub fn SIsMember<T: Arg, M: Arg>(
    key: T,
    member: M,
) -> utils::BoolOutputCommand<(&'static str, T, M)> {
    utils::BoolOutputCommand(("SISMEMBER", key, member))
}

/// SMISMEMBER redis command
///
/// Returns whether each member is a member of the set stored at `key`.
pub fn SMIsMember<T: Arg, M: Arg>(key: T, member: M) -> SMIsMemberCommand {
    let mut args = ArgList::new("SMISMEMBER");
    args.push(key);
    args.push(member);
    SMIsMemberCommand(args)
}

/// SMISMEMBER command, see [`SMIsMember`](crate::cmd::SMIsMember)
pub struct SMIsMemberCommand(ArgList);

impl SMIsMemberCommand {
    /// Add a member to check.
    pub fn member<M: Arg>(mut self, member: M) -> Self {
        self.0.push(member);
        self
    }

    /// Add more members to check.
    pub fn members<M: Arg>(mut self, other: impl IntoIterator<Item = M>) -> Self {
        for member in other {
            self.0.push(member);
        }
        self
    }
}

impl Command for SMIsMemberCommand {
    type Output = Vec<bool>;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_bool_list(frame)
    }
}

/// SCARD redis command
///
/// Returns the number of members of the set stored at `key`.
pub fn SCard<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("SCARD", key))
}

/// SPOP redis command
///
/// Removes and returns a random member of the set stored at `key`, `None`
/// if the key does not exist.
pub fn SPop<T: Arg>(key: T) -> utils::BulkOutputCommand<(&'static str, T)> {
    utils::BulkOutputCommand(("SPOP", key))
}

/// SRANDMEMBER redis command
///
/// Returns a random member of the set stored at `key`, `None` if the key
/// does not exist.
pub fn SRandMember<T: Arg>(key: T) -> utils::BulkOutputCommand<(&'static str, T)> {
    utils::BulkOutputCommand(("SRANDMEMBER", key))
}

/// SINTER redis command
///
/// Returns the members of the intersection of all the given sets.
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
///     redis.exec(cmd::SAdd(&key1, "a").members(["b", "c"])).await?;
///     redis.exec(cmd::SAdd(&key2, "b").members(["c", "d"])).await?;
///
///     let mut members = redis.exec(cmd::SInter(&key1).key(&key2)).await?;
///     members.sort();
///     assert_eq!(members, ["b", "c"]);
///
///     redis.exec(cmd::Del(&key1).key(&key2)).await?;
///     Ok(())
/// }
/// ```
pub fn SInter<T: Arg>(key: T) -> SetOpCommand {
    SetOpCommand::new("SINTER", key)
}

/// SUNION redis command
///
/// Returns the members of the union of all the given sets.
pub fn SUnion<T: Arg>(key: T) -> SetOpCommand {
    SetOpCommand::new("SUNION", key)
}

/// SDIFF redis command
///
/// Returns the members of the difference between the first set and all
/// the successive sets.
pub fn SDiff<T: Arg>(key: T) -> SetOpCommand {
    SetOpCommand::new("SDIFF", key)
}

/// SINTER, SUNION and SDIFF command, see [`SInter`](crate::cmd::SInter),
/// [`SUnion`](crate::cmd::SUnion) and [`SDiff`](crate::cmd::SDiff)
pub struct SetOpCommand(ArgList);

impl SetOpCommand {
    fn new<T: Arg>(name: &'static str, key: T) -> Self {
        let mut args = ArgList::new(name);
        args.push(key);
        SetOpCommand(args)
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

impl Command for SetOpCommand {
    type Output = Vec<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_list(frame)
    }
}

/// SSCAN redis command
///
/// Incrementally iterates over members of the set stored at `key`.
/// See [`Scan`](crate::cmd::Scan) for the iteration semantics.
pub fn SScan<T: Arg>(key: T, cursor: u64) -> utils::ScanCommand<Bytes> {
    let mut args = ArgList::new("SSCAN");
    args.push(key);
    args.push(cursor);
    utils::ScanCommand::new(args)
}
