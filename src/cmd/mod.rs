//! Redis commands
#![allow(non_snake_case)]

use super::errors::CommandError;
use super::{Request, Response};

mod auth;
mod get;

pub use self::auth::Auth;
pub use self::get::Get;

pub trait Command {
    type Output;

    fn to_request(self) -> Request;

    fn to_output(val: Response) -> Result<Self::Output, CommandError>;
}
