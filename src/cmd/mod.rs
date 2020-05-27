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

pub trait Command {
    type Output;

    fn to_request(self) -> Request;

    fn to_output(val: Response) -> Result<Self::Output, CommandError>;
}

pub mod dev {
    pub use super::auth::AuthCommand;
    pub use super::keys::KeysCommand;
    pub use super::lists::{LIndexCommand, LPopCommand, LPushCommand};
    pub use super::strings::{GetCommand, SetCommand};
}
