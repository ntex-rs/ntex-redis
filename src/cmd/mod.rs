//! Redis commands
#![allow(non_snake_case, clippy::wrong_self_convention)]

use super::codec::{Frame, Writer};
use super::errors::CommandError;

mod auth;
mod connection;
mod hashes;
mod keys;
mod lists;
mod pubsub;
mod server;
mod sets;
mod sorted_sets;
mod strings;
mod utils;

pub use self::auth::Auth;
pub use self::connection::{ClientGetName, ClientSetName, Echo, Ping, Reset, Select};
pub use self::hashes::{
    HDel, HExists, HGet, HGetAll, HIncrBy, HIncrByFloat, HKeys, HLen, HMGet, HScan, HSet, HSetNx,
    HStrLen, HVals,
};
pub use self::keys::{
    Del, Exists, Expire, ExpireAt, Keys, PExpire, PExpireAt, PTtl, Persist, Rename, RenameNx, Scan,
    Touch, Ttl, TtlResult, Type, Unlink,
};
pub use self::lists::{
    LIndex, LInsert, LLen, LMove, LPop, LPos, LPush, LRange, LRem, LSet, LTrim, RPop, RPush, Side,
};
pub use self::pubsub::{
    PSubscribe, PUnSubscribe, Publish, SPublish, SSubscribe, SUnSubscribe, Subscribe,
    SubscribeItem, UnSubscribe,
};
pub use self::server::{DbSize, FlushAll, FlushDb, Info};
pub use self::sets::{
    SAdd, SCard, SDiff, SInter, SIsMember, SMIsMember, SMembers, SPop, SRandMember, SRem, SScan,
    SUnion,
};
pub use self::sorted_sets::{
    ZAdd, ZCard, ZCount, ZIncrBy, ZPopMax, ZPopMin, ZRange, ZRank, ZRem, ZRevRank, ZScan, ZScore,
};
pub use self::strings::{
    Append, Decr, DecrBy, Get, GetDel, GetEx, GetRange, Incr, IncrBy, IncrByFloat, MGet, MSet, Set,
    SetRange, StrLen,
};

/// Trait implemented by types that can be used as redis commands
///
/// Commands are executed with [`Client::exec`](crate::Client::exec) or
/// [`SimpleClient::exec`](crate::SimpleClient::exec). Implement this trait
/// to add commands that are not provided by this crate.
///
/// ```rust
/// use ntex_redis::{cmd::Command, codec::{Frame, Writer}, errors::CommandError};
///
/// /// OBJECT FREQ redis command
/// struct ObjectFreq(String);
///
/// impl Command for ObjectFreq {
///     type Output = i64;
///
///     fn encode(self, w: &mut Writer<'_>) {
///         w.args(("OBJECT", "FREQ", self.0));
///     }
///
///     fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
///         frame.int()
///     }
/// }
/// ```
pub trait Command {
    /// Command output type
    type Output;

    /// Write the command request
    ///
    /// The request must be a single array of bulk strings, see [`Writer`].
    fn encode(self, w: &mut Writer<'_>);

    /// Decode command output from the response frame
    ///
    /// Redis error responses are returned as [`CommandError::Error`]
    /// before `decode` is called.
    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError>;
}

pub mod commands {
    //! Command types returned by the command constructors in [`cmd`](super).
    //!
    //! Some types provide builder methods to set additional command
    //! arguments, e.g. [`SetCommand::expire_secs`].
    pub use super::auth::AuthCommand;
    pub use super::connection::{PingCommand, ResetCommand, SelectCommand};
    pub use super::hashes::{HDelCommand, HGetAllCommand, HMGetCommand, HSetCommand};
    pub use super::keys::{KeysCommand, KeysPatternCommand, PTtlCommand, TtlCommand};
    pub use super::lists::{LInsertCommand, LPosCommand, LPushCommand};
    pub use super::pubsub::{PubSubCommand, SubscribeOutputCommand, UnSubscribeOutputCommand};
    pub use super::server::{FlushCommand, InfoCommand};
    pub use super::sets::{MembersCommand, SMIsMemberCommand, SetOpCommand};
    pub use super::sorted_sets::{
        ZAddCommand, ZPopCommand, ZRangeCommand, ZRangeWithScoresCommand, ZRankCommand,
        ZScoreCommand,
    };
    pub use super::strings::{GetExCommand, MGetCommand, MSetCommand, SetCommand};
    pub use super::utils::{
        BoolOutputCommand, BulkOutputCommand, BytesOutputCommand, FloatOutputCommand,
        IntOutputCommand, ListOutputCommand, OkOutputCommand, ScanCommand, StringOutputCommand,
    };
}
