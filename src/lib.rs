//! Redis client for ntex framework.
mod client;
pub mod cmd;
mod codec;
mod connector;
pub mod errors;
mod transport;

pub use self::client::Client;
pub use self::codec::{Codec, Request, Response};
pub use self::connector::RedisConnector;

/// Macro to create a RESP array, useful for preparing commands to send.  Elements can be any type, or a mixture
/// of types, that satisfy `Into<Value>`.
///
/// As a general rule, if a value is moved, the data can be deconstructed (if appropriate, e.g. String) and the raw
/// data moved into the corresponding `Value`.  If a reference is provided, the data will be copied instead.
///
/// # Examples
///
/// ```
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
/// ```
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
        $crate::Request::Array(vec![$($e.into(),)*])
    }}
}
