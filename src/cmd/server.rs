use ntex_bytes::ByteString;

use super::{Command, CommandError, utils};
use crate::codec::{Arg, ArgList, Frame, Writer};

/// FLUSHDB redis command
///
/// Deletes all the keys of the currently selected database.
pub fn FlushDb() -> FlushCommand {
    FlushCommand {
        name: "FLUSHDB",
        asynchronous: false,
    }
}

/// FLUSHALL redis command
///
/// Deletes all the keys of all the existing databases.
pub fn FlushAll() -> FlushCommand {
    FlushCommand {
        name: "FLUSHALL",
        asynchronous: false,
    }
}

/// FLUSHDB and FLUSHALL command, see [`FlushDb`](crate::cmd::FlushDb)
/// and [`FlushAll`](crate::cmd::FlushAll)
pub struct FlushCommand {
    name: &'static str,
    asynchronous: bool,
}

impl FlushCommand {
    /// Flush the database asynchronously
    pub fn asynchronous(mut self) -> Self {
        self.asynchronous = true;
        self
    }
}

impl Command for FlushCommand {
    type Output = ();

    fn encode(self, w: &mut Writer<'_>) {
        if self.asynchronous {
            w.args((self.name, "ASYNC"));
        } else {
            w.args((self.name,));
        }
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        utils::read_ok(frame)
    }
}

/// DBSIZE redis command
///
/// Returns the number of keys in the currently selected database.
pub fn DbSize() -> utils::IntOutputCommand<(&'static str,)> {
    utils::IntOutputCommand(("DBSIZE",))
}

/// INFO redis command
///
/// Returns information and statistics about the server. Use
/// [`InfoCommand::section`] to select specific sections.
///
/// ```rust
/// use ntex_redis::{cmd, RedisConnector};
///
/// #[ntex::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let redis = RedisConnector::new("127.0.0.1:6379").connect().await?;
///
///     let info = redis.exec(cmd::Info().section("server")).await?;
///     assert!(info.contains("redis_version:"));
///     Ok(())
/// }
/// ```
pub fn Info() -> InfoCommand {
    InfoCommand(ArgList::new("INFO"))
}

/// INFO command, see [`Info`](crate::cmd::Info)
pub struct InfoCommand(ArgList);

impl InfoCommand {
    /// Add a section to return, e.g. `server`, `memory` or `all`
    pub fn section<T: Arg>(mut self, section: T) -> Self {
        self.0.push(section);
        self
    }
}

impl Command for InfoCommand {
    type Output = ByteString;

    fn encode(self, w: &mut Writer<'_>) {
        self.0.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.string()
    }
}
