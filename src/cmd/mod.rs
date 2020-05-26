//! Redis commands
#![allow(non_snake_case)]

use super::errors::CommandError;
use super::{Request, Response};

mod auth;
mod get;
mod set;

pub use self::auth::Auth;
pub use self::get::Get;
pub use self::set::Set;

pub trait Command {
    type Output;

    fn to_request(self) -> Request;

    fn to_output(val: Response) -> Result<Self::Output, CommandError>;
}

pub mod dev {
    pub use super::auth::AuthCommand;
    pub use super::get::GetCommand;
    pub use super::set::SetCommand;
}
