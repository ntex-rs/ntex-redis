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
mod strings;
mod utils;

pub use self::auth::Auth;
pub use self::connection::{Ping, Reset, Select};
pub use self::hashes::{HDel, HGet, HGetAll, HIncrBy, HLen, HSet};
pub use self::keys::{Del, Exists, Expire, ExpireAt, Keys, Ttl, TtlResult};
pub use self::lists::{LIndex, LPop, LPush, RPop, RPush};
pub use self::pubsub::{
    PSubscribe, PUnSubscribe, Publish, SPublish, SSubscribe, SUnSubscribe, Subscribe,
    SubscribeItem, UnSubscribe,
};
pub use self::strings::{Get, IncrBy, Set};

/// Trait implemented by types that can be used as redis commands
///
/// Commands are executed with [`Client::exec`](crate::Client::exec) or
/// [`SimpleClient::exec`](crate::SimpleClient::exec). Implement this trait
/// to add commands that are not provided by this crate.
///
/// ```rust
/// use ntex_redis::{cmd::Command, codec::{Frame, Writer}, errors::CommandError};
///
/// /// STRLEN redis command
/// struct StrLen(String);
///
/// impl Command for StrLen {
///     type Output = i64;
///
///     fn encode(self, w: &mut Writer<'_>) {
///         w.args(("STRLEN", self.0));
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
    pub use super::hashes::{HDelCommand, HGetAllCommand, HSetCommand};
    pub use super::keys::{KeysCommand, KeysPatternCommand, TtlCommand};
    pub use super::lists::LPushCommand;
    pub use super::pubsub::{PubSubCommand, SubscribeOutputCommand, UnSubscribeOutputCommand};
    pub use super::strings::SetCommand;
    pub use super::utils::{BoolOutputCommand, BulkOutputCommand, IntOutputCommand};
}
