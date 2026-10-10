//! Redis client for ntex framework.
//!
//! Use [`RedisConnector`] to connect to a redis server. It creates one of two clients:
//!
//! * [`Client`] - shared client. It is cheap to clone, all clones use one
//!   connection and commands are pipelined.
//! * [`SimpleClient`] - executes one command at a time. It supports pub/sub,
//!   see [`SimpleClient::subscribe`].
//!
//! Commands are created with the functions in the [`cmd`] module. Custom
//! commands can be added by implementing the [`cmd::Command`] trait.
//!
//! This crate does not select an async runtime. Applications must enable
//! one of the ntex runtime features, e.g. `ntex/tokio`.
//!
//! # Example
//!
//! ```rust
//! use ntex_redis::{cmd, RedisConnector};
//! # use rand::{Rng, rng};
//! # use rand::distr::Alphanumeric;
//! # fn gen_random_key() -> String {
//! # let key: String = rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect();
//! # key
//! # }
//!
//! #[ntex::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
//!     let key = gen_random_key();
//!
//!     // create list with one value
//!     redis.exec(cmd::LPush(&key, "value")).await?;
//!
//!     // get value by index
//!     let value = redis.exec(cmd::LIndex(&key, 0)).await?;
//!     assert_eq!(value.unwrap(), "value");
//!
//!     // remove key
//!     redis.exec(cmd::Del(&key)).await?;
//!
//!     Ok(())
//! }
//! ```
#![allow(clippy::return_self_not_must_use)]

mod client;
pub mod cmd;
pub mod codec;
mod connector;
pub mod errors;
mod simple;

pub use self::client::Client;
pub use self::connector::RedisConnector;
pub use self::simple::{SimpleClient, SubscriptionClient};

/// Macro to create a request array, useful for preparing commands to send. Elements can be any type, or a mixture
/// of types, that satisfy `Into<Request>`.
///
/// # Examples
///
/// ```rust
/// use ntex_redis::array;
///
/// let value = format!("something_{}", 123);
/// let command = array!["SET", "key_name", value];
/// ```
///
/// For variable length Redis commands:
///
/// ```rust
/// use ntex_redis::array;
///
/// let data = vec!["data", "from", "somewhere", "else"];
/// let command = array!["RPUSH", "mykey"].extend(data);
/// ```
#[macro_export]
macro_rules! array {
    ($($e:expr),*) => {{
        $crate::codec::Request::Array(vec![$($e.into(),)*])
    }}
}

#[cfg(test)]
pub fn gen_random_key() -> String {
    use rand::distr::Alphanumeric;
    use rand::{Rng, rng};

    let key: String = rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    key
}
