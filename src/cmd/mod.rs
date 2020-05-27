//! Redis commands
#![allow(non_snake_case)]

use super::codec::{Request, Response};
use super::errors::CommandError;

mod auth;
mod keys;
mod lists;
mod strings;

pub use self::auth::Auth;
pub use self::keys::{Del, Exists};
pub use self::lists::{LIndex, LPop, LPush, RPop, RPush};
pub use self::strings::{Get, Set};

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
    pub use super::keys::KeysCommand;
    pub use super::lists::{LIndexCommand, LPopCommand, LPushCommand};
    pub use super::strings::{GetCommand, SetCommand};
}
