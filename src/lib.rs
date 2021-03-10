//! Redis client for ntex framework.
//!
//! # Example
//!
//! ```rust
//! use ntex_redis::{cmd, RedisConnector};
//! # use rand::{thread_rng, Rng};
//! # use rand::distributions::Alphanumeric;
//! # fn gen_random_key() -> String {
//! # let key: String = thread_rng().sample_iter(&Alphanumeric).take(12).collect();
//! # key
//! # }
//!
//! #[ntex::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
//!     let key = gen_random_key();
//!
//!     // create list with one value
//!     redis.exec(cmd::LPush(&key, "value"));
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
mod client;
pub mod cmd;
pub mod codec;
mod connector;
pub mod errors;
mod simple;
mod transport;

pub use self::client::{Client, CommandResult};
pub use self::connector::RedisConnector;
pub use self::simple::SimpleClient;

/// Macro to create a request array, useful for preparing commands to send. Elements can be any type, or a mixture
/// of types, that satisfy `Into<Request>`.
///
/// # Examples
///
/// ```rust
/// #[macro_use]
/// extern crate ntex_redis;
///
/// fn main() {
///     let value = format!("something_{}", 123);
///     array!["SET", "key_name", value];
/// }
/// ```
///
/// For variable length Redis commands:
///
/// ```rust
/// #[macro_use]
/// extern crate ntex_redis;
///
/// fn main() {
///     let data = vec!["data", "from", "somewhere", "else"];
///     let command = array!["RPUSH", "mykey"].extend(data);
/// }
/// ```
#[macro_export]
macro_rules! array {
    ($($e:expr),*) => {{
        $crate::codec::Request::Array(vec![$($e.into(),)*])
    }}
}

#[cfg(test)]
pub fn gen_random_key() -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let key: String = thread_rng().sample_iter(&Alphanumeric).take(12).collect();
    key
}
