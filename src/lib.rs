//! Redis client for ntex framework.
mod client;
pub mod cmd;
pub mod codec;
mod connector;
pub mod errors;
mod simple;
mod transport;

pub use self::client::Client;
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
