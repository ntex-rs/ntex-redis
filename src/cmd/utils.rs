use std::{marker::PhantomData, str};

use ntex_bytes::{ByteString, Bytes};

use super::{Command, CommandError};
use crate::codec::{Arg, ArgList, Args, Frame, Writer};

/// Generic command that returns an optional bulk string
///
/// Output is `None` if the value does not exist.
pub struct BulkOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for BulkOutputCommand<A> {
    type Output = Option<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.bulk()
    }
}

/// Generic command that returns a bulk string
pub struct BytesOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for BytesOutputCommand<A> {
    type Output = Bytes;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.bytes()
    }
}

/// Generic command that returns a string
pub struct StringOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for StringOutputCommand<A> {
    type Output = ByteString;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.string()
    }
}

/// Generic command that returns an integer
pub struct IntOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for IntOutputCommand<A> {
    type Output = i64;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.int()
    }
}

/// Generic command that returns a floating point number
pub struct FloatOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for FloatOutputCommand<A> {
    type Output = f64;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        frame.float()
    }
}

/// Generic command that returns a boolean
///
/// Redis integer reply `1` is converted to `true`, `0` to `false`.
pub struct BoolOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for BoolOutputCommand<A> {
    type Output = bool;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        read_bool(frame)
    }
}

/// Generic command that expects `OK` reply
pub struct OkOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for OkOutputCommand<A> {
    type Output = ();

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        read_ok(frame)
    }
}

/// Generic command that returns a list of bulk strings
pub struct ListOutputCommand<A>(pub(crate) A);

impl<A: Args> Command for ListOutputCommand<A> {
    type Output = Vec<Bytes>;

    fn encode(self, w: &mut Writer<'_>) {
        w.args(self.0);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        read_list(frame)
    }
}

/// SCAN, SSCAN, HSCAN and ZSCAN command
///
/// See [`Scan`](crate::cmd::Scan), [`SScan`](crate::cmd::SScan),
/// [`HScan`](crate::cmd::HScan) and [`ZScan`](crate::cmd::ZScan).
///
/// Output is the next cursor and a batch of items, iteration is complete
/// when returned cursor is `0`.
pub struct ScanCommand<I> {
    args: ArgList,
    _t: PhantomData<fn() -> I>,
}

impl<I> ScanCommand<I> {
    pub(crate) fn new(args: ArgList) -> Self {
        ScanCommand {
            args,
            _t: PhantomData,
        }
    }

    /// Only return elements that match the glob-style pattern
    pub fn pattern<T: Arg>(mut self, pattern: T) -> Self {
        self.args.push("MATCH");
        self.args.push(pattern);
        self
    }

    /// Amount of work that should be done at every call, default is 10
    ///
    /// It is a hint, redis may return more or fewer elements.
    pub fn count(mut self, count: usize) -> Self {
        self.args.push("COUNT");
        self.args.push(count);
        self
    }
}

impl ScanCommand<Bytes> {
    /// Only return keys of the given type, e.g. `string`, `list` or `hash`
    ///
    /// Supported by [`Scan`](crate::cmd::Scan) only.
    pub fn key_type<T: Arg>(mut self, key_type: T) -> Self {
        self.args.push("TYPE");
        self.args.push(key_type);
        self
    }
}

impl Command for ScanCommand<Bytes> {
    type Output = (u64, Vec<Bytes>);

    fn encode(self, w: &mut Writer<'_>) {
        self.args.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok((read_cursor(frame)?, read_list(frame)?))
    }
}

impl Command for ScanCommand<(Bytes, Bytes)> {
    type Output = (u64, Vec<(Bytes, Bytes)>);

    fn encode(self, w: &mut Writer<'_>) {
        self.args.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok((read_cursor(frame)?, read_pairs(frame, Frame::bytes)?))
    }
}

impl Command for ScanCommand<(Bytes, f64)> {
    type Output = (u64, Vec<(Bytes, f64)>);

    fn encode(self, w: &mut Writer<'_>) {
        self.args.write(w);
    }

    fn decode(frame: &mut Frame) -> Result<Self::Output, CommandError> {
        Ok((read_cursor(frame)?, read_pairs(frame, Frame::float)?))
    }
}

/// Read integer `0` or `1` as bool
pub(crate) fn read_bool(frame: &mut Frame) -> Result<bool, CommandError> {
    match frame.int()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(frame.unexpected("i64 value cannot be represented as bool")),
    }
}

/// Read `OK` simple string
pub(crate) fn read_ok(frame: &mut Frame) -> Result<(), CommandError> {
    if frame.peek()? == b'+' && frame.string()? == "OK" {
        Ok(())
    } else {
        Err(frame.unexpected("Unexpected value"))
    }
}

/// Read array header, nil array is an error
fn read_array(frame: &mut Frame) -> Result<usize, CommandError> {
    frame
        .array()?
        .ok_or_else(|| frame.unexpected("Cannot be converted into a vector"))
}

/// Read array of bulk strings
pub(crate) fn read_list(frame: &mut Frame) -> Result<Vec<Bytes>, CommandError> {
    let len = read_array(frame)?;
    let mut items = Vec::with_capacity(len);
    for _ in 0..len {
        items.push(frame.bytes()?);
    }
    Ok(items)
}

/// Read array of optional bulk strings
pub(crate) fn read_opt_list(frame: &mut Frame) -> Result<Vec<Option<Bytes>>, CommandError> {
    let len = read_array(frame)?;
    let mut items = Vec::with_capacity(len);
    for _ in 0..len {
        items.push(frame.bulk()?);
    }
    Ok(items)
}

/// Read array of `0` or `1` integers
pub(crate) fn read_bool_list(frame: &mut Frame) -> Result<Vec<bool>, CommandError> {
    let len = read_array(frame)?;
    let mut items = Vec::with_capacity(len);
    for _ in 0..len {
        items.push(read_bool(frame)?);
    }
    Ok(items)
}

/// Read flat array of bulk string and value pairs
pub(crate) fn read_pairs<V>(
    frame: &mut Frame,
    f: fn(&mut Frame) -> Result<V, CommandError>,
) -> Result<Vec<(Bytes, V)>, CommandError> {
    let len = read_array(frame)?;
    if len % 2 != 0 {
        return Err(frame.unexpected("Cannot convert an odd number of elements into pairs"));
    }
    let mut items = Vec::with_capacity(len / 2);
    for _ in 0..len / 2 {
        let item = frame.bytes()?;
        items.push((item, f(frame)?));
    }
    Ok(items)
}

/// Read scan reply header and cursor
fn read_cursor(frame: &mut Frame) -> Result<u64, CommandError> {
    if read_array(frame)? != 2 {
        return Err(frame.unexpected("Invalid scan reply"));
    }
    let cursor = frame.bytes()?;
    str::from_utf8(&cursor)
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| frame.unexpected("Invalid scan cursor"))
}
