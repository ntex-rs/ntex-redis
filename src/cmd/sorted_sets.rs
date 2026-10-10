use ntex_bytes::Bytes;

use super::{Command, CommandError, sets::MembersCommand, utils};
use crate::codec::{Arg, ArgList, Frame, Writer};

/// ZADD redis command
///
/// Adds the member with the specified score to the sorted set stored at `key`.
/// Use [`ZAddCommand::entry`] to add multiple members. Returns the number of
/// added members, or the number of changed members if [`ZAddCommand::ch`] is set.
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
///     redis.exec(cmd::ZAdd(&key, 1.0, "a").entry(2.0, "b").entry(3.5, "c")).await?;
///
///     assert_eq!(redis.exec(cmd::ZScore(&key, "c")).await?, Some(3.5));
///     assert_eq!(redis.exec(cmd::ZRank(&key, "b")).await?, Some(1));
///     assert_eq!(redis.exec(cmd::ZRange(&key, 0, -1)).await?, ["a", "b", "c"]);
///
///     let items = redis.exec(cmd::ZRange(&key, "(1", "+inf").by_score().with_scores()).await?;
///     assert_eq!(items, [("b".into(), 2.0), ("c".into(), 3.5)]);
///
///     redis.exec(cmd::Del(&key)).await?;
///     Ok(())
/// }
/// ```
pub fn ZAdd<T: Arg, M: Arg>(key: T, score: f64, member: M) -> ZAddCommand<T> {
    let mut entries = ArgList::new("ZADD");
    entries.push(score);
    entries.push(member);
    ZAddCommand {
        key,
        entries,
        cond: None,
        cmp: None,
        ch: false,
    }
}

/// ZADD command, see [`ZAdd`](crate::cmd::ZAdd)
pub struct ZAddCommand<T> {
    key: T,
    entries: ArgList,
    cond: Option<&'static str>,
    cmp: Option<&'static str>,
    ch: bool,
}

impl<T> ZAddCommand<T> {
    /// Add another member with score
    pub fn entry<M: Arg>(mut self, score: f64, member: M) -> Self {
        self.add_entry(score, member);
        self
    }

    /// Add another member with score, by mutable reference
    pub fn add_entry<M: Arg>(&mut self, score: f64, member: M) {
        self.entries.push(score);
        self.entries.push(member);
    }

    /// Only update members that already exist, never add new members
    pub fn if_exists(mut self) -> Self {
        self.cond = Some("XX");
        self
    }

    /// Only add new members, never update existing members
    pub fn if_not_exists(mut self) -> Self {
        self.cond = Some("NX");
        self
    }

    /// Only update existing members if new score is greater than current score
    pub fn gt(mut self) -> Self {
        self.cmp = Some("GT");
        self
    }

    /// Only update existing members if new score is less than current score
    pub fn lt(mut self) -> Self {
        self.cmp = Some("LT");
        self
    }

    /// Return the number of changed members, added and updated
    pub fn ch(mut self) -> Self {
        self.ch = true;
        self
    }
}

impl<T: Arg> Command for ZAddCommand<T> {
    type Output = usize;

    fn encode(self, w: &mut Writer<'_>) {
        let flags = usize::from(self.cond.is_some())
            + usize::from(self.cmp.is_some())
            + usize::from(self.ch);
        w.array(2 + flags + self.entries.len());
        w.arg("ZADD");
        w.arg(self.key);
        if let Some(cond) = self.cond {
            w.arg(cond);
        }
        if let Some(cmp) = self.cmp {
            w.arg(cmp);
        }
        if self.ch {
            w.arg("CH");
        }
        self.entries.write_args(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok(frame.int()? as usize)
    }
}

/// ZREM redis command
///
/// Removes the members from the sorted set stored at `key`. Returns
/// the number of members that were removed.
pub fn ZRem<T: Arg, M: Arg>(key: T, member: M) -> MembersCommand {
    MembersCommand::new("ZREM", key, member)
}

/// ZSCORE redis command
///
/// Returns the score of `member` in the sorted set stored at `key`, `None`
/// if the member or the key do not exist.
pub fn ZScore<T: Arg, M: Arg>(key: T, member: M) -> ZScoreCommand<T, M> {
    ZScoreCommand(key, member)
}

/// ZSCORE command, see [`ZScore`](crate::cmd::ZScore)
pub struct ZScoreCommand<T, M>(T, M);

impl<T: Arg, M: Arg> Command for ZScoreCommand<T, M> {
    type Output = Option<f64>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(("ZSCORE", self.0, self.1));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        if frame.is_nil() {
            frame.bulk().map(|_| None)
        } else {
            frame.float().map(Some)
        }
    }
}

/// ZINCRBY redis command
///
/// Increments the score of `member` in the sorted set stored at `key` by
/// `increment` and returns the new score. Missing member is added with
/// `increment` as its score.
pub fn ZIncrBy<T: Arg, M: Arg>(
    key: T,
    increment: f64,
    member: M,
) -> utils::FloatOutputCommand<(&'static str, T, f64, M)> {
    utils::FloatOutputCommand(("ZINCRBY", key, increment, member))
}

/// ZCARD redis command
///
/// Returns the number of members of the sorted set stored at `key`.
pub fn ZCard<T: Arg>(key: T) -> utils::IntOutputCommand<(&'static str, T)> {
    utils::IntOutputCommand(("ZCARD", key))
}

/// ZCOUNT redis command
///
/// Returns the number of members in the sorted set stored at `key` with
/// a score between `min` and `max`. Bounds are inclusive by default, use
/// `"(1.5"` for an exclusive bound and `"-inf"`, `"+inf"` for no bound.
pub fn ZCount<T: Arg, A: Arg, B: Arg>(
    key: T,
    min: A,
    max: B,
) -> utils::IntOutputCommand<(&'static str, T, A, B)> {
    utils::IntOutputCommand(("ZCOUNT", key, min, max))
}

/// ZRANK redis command
///
/// Returns the rank of `member` in the sorted set stored at `key`, with
/// scores ordered from low to high. Returns `None` if the member or the key
/// do not exist.
pub fn ZRank<T: Arg, M: Arg>(key: T, member: M) -> ZRankCommand<T, M> {
    ZRankCommand("ZRANK", key, member)
}

/// ZREVRANK redis command
///
/// Returns the rank of `member` in the sorted set stored at `key`, with
/// scores ordered from high to low. Returns `None` if the member or the key
/// do not exist.
pub fn ZRevRank<T: Arg, M: Arg>(key: T, member: M) -> ZRankCommand<T, M> {
    ZRankCommand("ZREVRANK", key, member)
}

/// ZRANK and ZREVRANK command, see [`ZRank`](crate::cmd::ZRank)
/// and [`ZRevRank`](crate::cmd::ZRevRank)
pub struct ZRankCommand<T, M>(&'static str, T, M);

impl<T: Arg, M: Arg> Command for ZRankCommand<T, M> {
    type Output = Option<usize>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args((self.0, self.1, self.2));
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        if frame.is_nil() {
            frame.bulk().map(|_| None)
        } else {
            Ok(Some(frame.int()? as usize))
        }
    }
}

/// ZRANGE redis command
///
/// Returns the members of the sorted set stored at `key` between `start`
/// and `stop`. By default `start` and `stop` are inclusive ranks, use
/// [`ZRangeCommand::by_score`] or [`ZRangeCommand::by_lex`] to select
/// members by score or lexicographical range. See [`ZAdd`] for an example.
pub fn ZRange<T: Arg, A: Arg, B: Arg>(key: T, start: A, stop: B) -> ZRangeCommand {
    let mut args = ArgList::new("ZRANGE");
    args.push(key);
    args.push(start);
    args.push(stop);
    ZRangeCommand(args)
}

/// ZRANGE command, see [`ZRange`](crate::cmd::ZRange)
pub struct ZRangeCommand(ArgList);

impl ZRangeCommand {
    /// Treat `start` and `stop` as scores, e.g. `1.5`, `"(1.5"` or `"-inf"`
    pub fn by_score(mut self) -> Self {
        self.0.push("BYSCORE");
        self
    }

    /// Treat `start` and `stop` as lexicographical bounds, e.g. `"[a"`, `"(b"` or `"-"`
    pub fn by_lex(mut self) -> Self {
        self.0.push("BYLEX");
        self
    }

    /// Reverse the ordering, from high to low scores
    ///
    /// With score or lexicographical range, `start` must be the higher bound.
    pub fn rev(mut self) -> Self {
        self.0.push("REV");
        self
    }

    /// Return `count` members starting at `offset`, requires score or
    /// lexicographical range. Negative `count` returns all members from `offset`.
    pub fn limit(mut self, offset: usize, count: i64) -> Self {
        self.0.push("LIMIT");
        self.0.push(offset);
        self.0.push(count);
        self
    }

    /// Return members together with their scores
    pub fn with_scores(mut self) -> ZRangeWithScoresCommand {
        self.0.push("WITHSCORES");
        ZRangeWithScoresCommand(self.0)
    }
}

impl Command for ZRangeCommand {
    type Output = Vec<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_list(frame)
    }
}

/// ZRANGE command with scores, see [`ZRangeCommand::with_scores`]
pub struct ZRangeWithScoresCommand(ArgList);

impl Command for ZRangeWithScoresCommand {
    type Output = Vec<(Bytes, f64)>;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_pairs(frame, Frame::float)
    }
}

/// ZPOPMIN redis command
///
/// Removes and returns the member with the lowest score from the sorted set
/// stored at `key`. Use [`ZPopCommand::count`] to pop multiple members.
pub fn ZPopMin<T: Arg>(key: T) -> ZPopCommand<T> {
    ZPopCommand {
        name: "ZPOPMIN",
        key,
        count: None,
    }
}

/// ZPOPMAX redis command
///
/// Removes and returns the member with the highest score from the sorted set
/// stored at `key`. Use [`ZPopCommand::count`] to pop multiple members.
pub fn ZPopMax<T: Arg>(key: T) -> ZPopCommand<T> {
    ZPopCommand {
        name: "ZPOPMAX",
        key,
        count: None,
    }
}

/// ZPOPMIN and ZPOPMAX command, see [`ZPopMin`](crate::cmd::ZPopMin)
/// and [`ZPopMax`](crate::cmd::ZPopMax)
pub struct ZPopCommand<T> {
    name: &'static str,
    key: T,
    count: Option<usize>,
}

impl<T> ZPopCommand<T> {
    /// Pop up to `count` members
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }
}

impl<T: Arg> Command for ZPopCommand<T> {
    type Output = Vec<(Bytes, f64)>;

    fn encode(self, w: &mut Writer<'_>) {
        if let Some(count) = self.count {
            w.args((self.name, self.key, count));
        } else {
            w.args((self.name, self.key));
        }
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_pairs(frame, Frame::float)
    }
}

/// ZSCAN redis command
///
/// Incrementally iterates over members and scores of the sorted set stored
/// at `key`. See [`Scan`](crate::cmd::Scan) for the iteration semantics.
pub fn ZScan<T: Arg>(key: T, cursor: u64) -> utils::ScanCommand<(Bytes, f64)> {
    let mut args = ArgList::new("ZSCAN");
    args.push(key);
    args.push(cursor);
    utils::ScanCommand::new(args)
}
