use ntex::util::ByteString;

use super::{Command, CommandError};
use crate::codec::{Request, Response};

/// SELECT redis command
///
/// Select the Redis logical database having the specified zero-based
/// numeric index.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///
///     // select db for current connection
///     let success = redis.exec(cmd::Select(1)).await?;
///
///     assert!(success);
///
///     Ok(())
/// }
/// ```
pub fn Select(db: u32) -> SelectCommand {
    SelectCommand(Request::Array(vec![
        Request::from_static("SELECT"),
        Request::BulkInteger(db as i64),
    ]))
}

pub struct SelectCommand(Request);

impl Command for SelectCommand {
    type Output = bool;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::String(val) => Ok(val == "OK"),
            _ => Ok(false),
        }
    }
}

/// PING redis command
///
/// This command is often used to test if a connection is still alive,
/// or to measure latency.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///
///     // ping connection
///     let response = redis.exec(cmd::Ping()).await?;
///
///     assert_eq!(&response, "PONG");
///
///     Ok(())
/// }
/// ```
pub fn Ping() -> PingCommand {
    PingCommand(Request::Array(vec![Request::from_static("PING")]))
}

pub struct PingCommand(Request);

impl Command for PingCommand {
    type Output = ByteString;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::String(val) => Ok(val),
            Response::Error(val) => Err(CommandError::Error(val)),
            _ => Err(CommandError::Output("Unknown response", val)),
        }
    }
}

/// RESET redis command
/// This command performs a full reset of the connection's server-side context, mimicking the effect of disconnecting and reconnecting again.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///
///     // reset connection
///     let response = redis.exec(cmd::Reset()).await?;
///
///     assert_eq!(&response, "RESET");
///
///     Ok(())
/// }
/// ```
pub fn Reset() -> ResetCommand {
    ResetCommand(Request::Array(vec![Request::from_static("RESET")]))
}
pub struct ResetCommand(Request);

impl Command for ResetCommand {
    type Output = ByteString;

    fn to_request(self) -> Request {
        self.0
    }

    fn to_output(val: Response) -> Result<Self::Output, CommandError> {
        match val {
            Response::String(val) => Ok(val),
            Response::Error(val) => Err(CommandError::Error(val)),
            _ => Err(CommandError::Output("Unknown response", val)),
        }
    }
}
