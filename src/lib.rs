//! Redis client for ntex framework.
mod codec;

pub use self::codec::{Codec, Value};

#[derive(Debug)]
pub enum Error {
    /// A RESP parsing error occurred
    Parse(String),

    /// A RESP deserialising error occurred
    Decode(&'static str, codec::Value),

    /// An IO error occurred
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

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
///     let command = array!["RPUSH", "mykey"].append(data);
/// }
/// ```
#[macro_export]
macro_rules! array {
    ($($e:expr),*) => {{
        $crate::Value::Array(vec![$($e.into(),)*])
    }}
}
