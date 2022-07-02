//! Redis commands
#![allow(non_snake_case, clippy::wrong_self_convention)]

use super::codec::{Request, Response};
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
pub trait Command {
    /// Command output type
    type Output;

    /// Convert command to a redis request
    fn to_request(self) -> Request;

    /// Create command response from a redis response
    fn to_output(val: Response) -> Result<Self::Output, CommandError>;
}

pub mod commands {
    //! Command implementations
    pub use super::auth::AuthCommand;
    pub use super::hashes::{HDelCommand, HGetAllCommand, HSetCommand};
    pub use super::keys::{KeysCommand, KeysPatternCommand, TtlCommand};
    pub use super::lists::LPushCommand;
    pub use super::pubsub::{PubSubCommand, SubscribeOutputCommand};
    pub use super::strings::SetCommand;
    pub use super::utils::{BulkOutputCommand, IntOutputCommand};
}
