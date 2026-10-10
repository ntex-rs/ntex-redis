//! Redis protocol codec
use std::{
    cell::Cell, cmp, collections::HashMap, convert::TryFrom, hash::BuildHasher, hash::Hash, str,
};

use ntex_bytes::{BufMut, BytePageSize, BytePages, ByteString, Bytes, BytesMut};
use ntex_codec::{Decoder, Encoder};
use ntex_util::hash_map::HashMap as HashbrownMap;

use super::{
    cmd::Command,
    errors::{CommandError, Error},
};

/// Codec to read/write redis values
///
/// Decoder keeps the scan state of a partially received response, a codec
/// instance must not be used for decoding of multiple streams.
#[derive(Debug, Default)]
pub struct Codec {
    scan: Scanner,
}

impl Codec {
    /// Create new codec
    pub fn new() -> Self {
        Self::default()
    }
}

impl Encoder for Codec {
    type Item = Request;
    type Error = Error;

    fn encode(&self, msg: Request, buf: &mut BytePages) -> Result<(), Self::Error> {
        Writer::new(buf).request(msg);
        Ok(())
    }
}

/// Writes redis requests to a write buffer.
///
/// A redis request is an array of bulk strings. Write the array header with
/// [`array`](Writer::array) followed by exactly that many arguments, or
/// write a tuple of arguments with [`args`](Writer::args).
///
/// ```rust
/// use ntex_bytes::BytePages;
/// use ntex_redis::codec::Writer;
///
/// let mut buf = BytePages::default();
/// let mut w = Writer::new(&mut buf);
/// w.args(("SET", "key", 10));
///
/// w.array(3);
/// w.arg("RPUSH");
/// w.arg("list");
/// w.arg(b"value");
///
/// assert_eq!(
///     buf.freeze(),
///     &b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$2\r\n10\r\n\
///        *3\r\n$5\r\nRPUSH\r\n$4\r\nlist\r\n$5\r\nvalue\r\n"[..]
/// );
/// ```
pub struct Writer<'a> {
    buf: &'a mut BytePages,
}

impl<'a> Writer<'a> {
    /// Create a writer that appends to `buf`
    pub fn new(buf: &'a mut BytePages) -> Self {
        Writer { buf }
    }

    /// Write an array header, it must be followed by `len` arguments
    pub fn array(&mut self, len: usize) {
        write_header(b'*', len as i64, self.buf);
    }

    /// Write an argument
    pub fn arg<T: Arg>(&mut self, val: T) {
        val.write_arg(self);
    }

    /// Write a request from a tuple of arguments
    pub fn args<T: Args>(&mut self, args: T) {
        self.array(T::LEN);
        args.write_args(self);
    }

    /// Write a request
    ///
    /// A request that is not an [`Array`](Request::Array) is sent as
    /// a single element array.
    pub fn request(&mut self, req: Request) {
        if !matches!(req, Request::Array(_)) {
            self.array(1);
        }
        encode_value(req, self.buf);
    }

    /// Write a bulk string, data is copied
    fn bulk(&mut self, data: &[u8]) {
        if data.len() <= SMALL_BULK {
            // header, data and CRLF in one write
            let mut out = [0u8; SMALL_BULK + 7];
            let len = data.len();
            let start = if len < 10 {
                out[..2].copy_from_slice(&[b'$', b'0' + len as u8]);
                2
            } else {
                out[..3].copy_from_slice(&[b'$', b'0' + (len / 10) as u8, b'0' + (len % 10) as u8]);
                3
            };
            out[start..start + 2].copy_from_slice(b"\r\n");
            out[start + 2..start + 2 + len].copy_from_slice(data);
            out[start + 2 + len..start + 4 + len].copy_from_slice(b"\r\n");
            self.buf.extend_from_slice(&out[..start + 4 + len]);
        } else {
            write_header(b'$', data.len() as i64, self.buf);
            self.buf.extend_from_slice(data);
            write_rn(self.buf);
        }
    }

    /// Write a bulk string, large data is added without copying
    fn bulk_bytes(&mut self, data: Bytes) {
        if data.len() <= SMALL_BULK {
            self.bulk(&data);
        } else {
            write_header(b'$', data.len() as i64, self.buf);
            self.buf.append(data);
            write_rn(self.buf);
        }
    }

    /// Write an integer as a bulk string
    fn bulk_int<I: itoa::Integer>(&mut self, val: I) {
        let mut buffer = itoa::Buffer::new();
        self.bulk(buffer.format(val).as_bytes());
    }
}

/// Bulk strings up to this size are written with a single copy
const SMALL_BULK: usize = 64;

/// Max read buffer reservation for an incomplete bulk string, unless more is already buffered
const MAX_RESERVE: usize = 1024 * 1024;

/// Buffered size of an incomplete response above which read buffer grows geometrically
const GROW_THRESHOLD: usize = 256 * 1024;

/// A command argument, written as a bulk string.
///
/// Implemented for strings, byte slices, [`Bytes`] and integers. Integers
/// are written as their decimal representation.
pub trait Arg {
    /// Write the argument
    fn write_arg(self, w: &mut Writer<'_>);
}

macro_rules! impl_arg_slice {
    ($($t:ty),*) => {$(
        impl Arg for $t {
            fn write_arg(self, w: &mut Writer<'_>) {
                w.bulk(AsRef::<[u8]>::as_ref(&self));
            }
        }
    )*}
}

impl_arg_slice!(&str, &&str, &String, String, &[u8], &Vec<u8>, Vec<u8>);

impl<const N: usize> Arg for &[u8; N] {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk(self);
    }
}

impl Arg for Bytes {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self);
    }
}

impl Arg for &Bytes {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self.clone());
    }
}

impl Arg for BytesMut {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self.freeze());
    }
}

impl Arg for ByteString {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self.into_bytes());
    }
}

impl Arg for &ByteString {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self.as_bytes().clone());
    }
}

impl Arg for BulkString {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self.0);
    }
}

impl Arg for &BulkString {
    fn write_arg(self, w: &mut Writer<'_>) {
        w.bulk_bytes(self.0.clone());
    }
}

macro_rules! impl_arg_int {
    ($($t:ty),*) => {$(
        impl Arg for $t {
            fn write_arg(self, w: &mut Writer<'_>) {
                w.bulk_int(self);
            }
        }
    )*}
}

impl_arg_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

/// A tuple of [`Arg`]s that forms a complete request.
pub trait Args {
    /// Number of arguments
    const LEN: usize;

    /// Write the arguments, without the array header
    fn write_args(self, w: &mut Writer<'_>);
}

macro_rules! impl_args {
    ($len:expr; $($t:ident),*) => {
        impl<$($t: Arg),*> Args for ($($t,)*) {
            const LEN: usize = $len;

            #[allow(non_snake_case)]
            fn write_args(self, w: &mut Writer<'_>) {
                let ($($t,)*) = self;
                $($t.write_arg(w);)*
            }
        }
    }
}

impl_args!(1; A);
impl_args!(2; A, B);
impl_args!(3; A, B, C);
impl_args!(4; A, B, C, D);
impl_args!(5; A, B, C, D, E);
impl_args!(6; A, B, C, D, E, F);
impl_args!(7; A, B, C, D, E, F, G);
impl_args!(8; A, B, C, D, E, F, G, H);

/// Command name and a variable number of arguments, written in advance
pub(crate) struct ArgList {
    name: &'static str,
    len: usize,
    buf: BytePages,
}

impl ArgList {
    pub(crate) fn new(name: &'static str) -> Self {
        ArgList {
            name,
            len: 0,
            buf: BytePages::new(BytePageSize::Size4),
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn set_name(&mut self, name: &'static str) {
        self.name = name;
    }

    pub(crate) fn push<T: Arg>(&mut self, val: T) {
        self.len += 1;
        val.write_arg(&mut Writer::new(&mut self.buf));
    }

    pub(crate) fn write(mut self, w: &mut Writer<'_>) {
        w.array(self.len + 1);
        w.arg(self.name);
        self.buf.move_to(w.buf);
    }
}

fn encode_value(msg: Request, buf: &mut BytePages) {
    match msg {
        Request::Array(ary) => {
            write_header(b'*', ary.len() as i64, buf);
            for v in ary {
                encode_value(v, buf);
            }
        }
        Request::BulkString(bstr) => {
            write_header(b'$', bstr.0.len() as i64, buf);
            buf.append(bstr.0);
            write_rn(buf);
        }
        Request::BulkStatic(bstr) => {
            write_header(b'$', bstr.len() as i64, buf);
            buf.append(Bytes::from_static(bstr));
            write_rn(buf);
        }
        Request::BulkInteger(i) => {
            let mut buffer = itoa::Buffer::new();
            let rendered = buffer.format(i);
            write_header(b'$', rendered.len() as i64, buf);
            buf.extend_from_slice(rendered.as_bytes());
            write_rn(buf);
        }
        Request::String(ref string) => {
            write_string(b'+', string, buf);
        }
        Request::Integer(val) => {
            // Simple integer are just the header
            write_header(b':', val, buf);
        }
    }
}

impl Decoder for Codec {
    type Item = Response;
    type Error = Error;

    fn decode(&self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(len) = self.scan.frame_len(buf)? {
            Frame::new(buf.split_to(len)).read_value().map(Some)
        } else {
            Ok(None)
        }
    }
}

/// Decoder that splits complete response frames from the read buffer
#[derive(Debug, Default)]
pub(crate) struct FrameCodec {
    scan: Scanner,
}

impl Decoder for FrameCodec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.scan.frame_len(buf)?.map(|len| buf.split_to(len)))
    }
}

/// Redis response reader
///
/// Frame holds one complete redis response, values are read sequentially
/// with typed readers, see [`Command::decode`].
///
/// Typed readers return [`CommandError::Error`] for redis error values and
/// [`CommandError::Output`] with the whole response if the value has
/// an unexpected type. Both nil forms, `$-1` and `*-1`, are accepted
/// as nil by [`bulk`](Frame::bulk) and [`array`](Frame::array).
#[derive(Debug, Clone)]
pub struct Frame {
    buf: Bytes,
    pos: usize,
}

impl Frame {
    /// Create reader for a response frame
    #[inline]
    pub fn new(buf: Bytes) -> Self {
        Frame { buf, pos: 0 }
    }

    /// Returns true if the next value is nil
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.buf.get(self.pos), Some(b'$' | b'*'))
            && self.buf.get(self.pos + 1..self.pos + 3) == Some(b"-1")
    }

    /// Read integer value
    #[inline]
    pub fn int(&mut self) -> Result<i64, CommandError> {
        match self.peek()? {
            b':' => Ok(self.header()?),
            b'-' => Err(self.read_error()),
            _ => Err(self.unexpected("Not an integer")),
        }
    }

    /// Read bulk string value, returns `None` for nil
    #[inline]
    pub fn bulk(&mut self) -> Result<Option<Bytes>, CommandError> {
        match self.peek()? {
            b'$' => match self.header()? {
                -1 => Ok(None),
                size => Ok(Some(self.read_bulk(size)?)),
            },
            b'*' if self.is_nil() => {
                self.header()?;
                Ok(None)
            }
            b'-' => Err(self.read_error()),
            _ => Err(self.unexpected("Not a bytes object")),
        }
    }

    /// Read non-nil bulk string value
    #[inline]
    pub fn bytes(&mut self) -> Result<Bytes, CommandError> {
        match self.peek()? {
            b'$' => match self.header()? {
                -1 => Err(self.unexpected("Not a bytes object")),
                size => Ok(self.read_bulk(size)?),
            },
            b'-' => Err(self.read_error()),
            _ => Err(self.unexpected("Not a bytes object")),
        }
    }

    /// Read simple string or utf-8 bulk string value
    #[inline]
    pub fn string(&mut self) -> Result<ByteString, CommandError> {
        match self.peek()? {
            b'+' => Ok(to_string(self.read_line()?)?),
            b'$' => match self.header()? {
                -1 => Err(self.unexpected("Cannot convert into a string")),
                size => {
                    let data = self.read_bulk(size)?;
                    ByteString::try_from(data)
                        .map_err(|()| self.unexpected("Cannot convert into a string"))
                }
            },
            b'-' => Err(self.read_error()),
            _ => Err(self.unexpected("Cannot convert into a string")),
        }
    }

    /// Read array header, returns number of elements or `None` for nil
    ///
    /// Array elements must be read after the header.
    #[inline]
    pub fn array(&mut self) -> Result<Option<usize>, CommandError> {
        match self.peek()? {
            b'*' => match self.header()? {
                -1 => Ok(None),
                size => Ok(Some(self.array_size(size)?)),
            },
            b'$' if self.is_nil() => {
                self.header()?;
                Ok(None)
            }
            b'-' => Err(self.read_error()),
            _ => Err(self.unexpected("Not an array")),
        }
    }

    /// Read any value
    ///
    /// Redis error values are returned as [`Response::Error`].
    pub fn value(&mut self) -> Result<Response, CommandError> {
        Ok(self.read_value()?)
    }

    /// Create [`CommandError::Output`] error with the whole response
    pub fn unexpected(&self, msg: &'static str) -> CommandError {
        let val = Frame::new(self.buf.clone())
            .read_value()
            .unwrap_or(Response::Nil);
        CommandError::Output(msg, val)
    }

    /// Decode command output, redis error response is returned as [`CommandError::Error`]
    pub(crate) fn decode<T: Command>(mut self) -> Result<T::Output, CommandError> {
        match self.peek()? {
            b'-' => Err(self.read_error()),
            _ => T::decode(&mut self),
        }
    }

    #[inline]
    pub(crate) fn peek(&self) -> Result<u8, Error> {
        self.buf.get(self.pos).copied().ok_or_else(incomplete)
    }

    /// Reads error value as [`CommandError::Error`]
    pub(crate) fn read_error(&mut self) -> CommandError {
        match self.read_line().and_then(to_string) {
            Ok(err) => CommandError::Error(err),
            Err(err) => err.into(),
        }
    }

    pub(crate) fn read_value(&mut self) -> Result<Response, Error> {
        match self.peek()? {
            b'$' => match self.header()? {
                -1 => Ok(Response::Nil),
                size => Ok(Response::Bytes(self.read_bulk(size)?)),
            },
            b'*' => match self.header()? {
                -1 => Ok(Response::Nil),
                size => {
                    let size = self.array_size(size)?;
                    let mut values = Vec::with_capacity(size);
                    for _ in 0..size {
                        values.push(self.read_value()?);
                    }
                    Ok(Response::Array(values))
                }
            },
            b':' => Ok(Response::Integer(self.header()?)),
            b'+' => Ok(Response::String(to_string(self.read_line()?)?)),
            b'-' => Ok(Response::Error(to_string(self.read_line()?)?)),
            b => Err(Error::Parse(format!("Unexpected byte: {b}"))),
        }
    }

    /// Reads integer after the type byte
    #[inline]
    fn header(&mut self) -> Result<i64, Error> {
        let (pos, val) = parse_int(&self.buf, self.pos + 1)?.ok_or_else(incomplete)?;
        self.pos = pos;
        Ok(val)
    }

    /// Validates array size, every element takes at least 3 bytes
    #[inline]
    fn array_size(&self, size: i64) -> Result<usize, Error> {
        usize::try_from(size)
            .ok()
            .filter(|size| *size <= (self.buf.len() - self.pos) / 3)
            .ok_or_else(|| Error::Parse(format!("Invalid array size: {size}")))
    }

    /// Reads line after the type byte
    #[inline]
    fn read_line(&mut self) -> Result<Bytes, Error> {
        let start = self.pos + 1;
        let end = find_crlf(&self.buf, start).ok_or_else(incomplete)?;
        self.pos = end + 2;
        Ok(self.buf.slice(start..end))
    }

    /// Reads bulk string payload of `size` bytes
    #[inline]
    fn read_bulk(&mut self, size: i64) -> Result<Bytes, Error> {
        let start = self.pos;
        let end = usize::try_from(size)
            .ok()
            .and_then(|size| start.checked_add(size))
            .filter(|end| *end < usize::MAX - 2)
            .ok_or_else(|| Error::Parse(format!("Invalid string size: {size}")))?;
        match self.buf.get(end..end + 2) {
            Some(b"\r\n") => {
                self.pos = end + 2;
                Ok(self.buf.slice(start..end))
            }
            Some(_) => Err(Error::Parse("Bulk string is not terminated by CRLF".into())),
            None => Err(incomplete()),
        }
    }
}

fn incomplete() -> Error {
    Error::Parse("Incomplete frame".into())
}

fn to_string(data: Bytes) -> Result<ByteString, Error> {
    ByteString::try_from(&data).map_err(|()| {
        Error::Parse(format!(
            "Not a valid string: {:?}",
            &data[..cmp::min(data.len(), 10)]
        ))
    })
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// A bulk string.
///
/// In Redis terminology a string is a byte-array, so this is stored as
/// [`Bytes`] to allow clients to interpret the bytes as appropriate.
pub struct BulkString(Bytes);

impl BulkString {
    /// Create bulk string from a static str, without copying
    pub fn from_static(data: &'static str) -> Self {
        BulkString(Bytes::from_static(data.as_ref()))
    }

    /// Create bulk string from a static byte slice, without copying
    pub fn from_bstatic(data: &'static [u8]) -> Self {
        BulkString(Bytes::from_static(data))
    }
}

impl From<ByteString> for BulkString {
    fn from(val: ByteString) -> BulkString {
        BulkString(val.into_bytes())
    }
}

impl From<String> for BulkString {
    fn from(val: String) -> BulkString {
        BulkString(Bytes::from(val))
    }
}

impl<'a> From<&'a String> for BulkString {
    fn from(val: &'a String) -> BulkString {
        BulkString(Bytes::copy_from_slice(val.as_ref()))
    }
}

impl<'a> From<&'a str> for BulkString {
    fn from(val: &'a str) -> BulkString {
        BulkString(Bytes::copy_from_slice(val.as_bytes()))
    }
}

impl<'a> From<&&'a str> for BulkString {
    fn from(val: &&'a str) -> BulkString {
        BulkString(Bytes::copy_from_slice(val.as_bytes()))
    }
}

impl From<Bytes> for BulkString {
    fn from(val: Bytes) -> BulkString {
        BulkString(val)
    }
}

impl From<BytesMut> for BulkString {
    fn from(val: BytesMut) -> BulkString {
        BulkString(val.freeze())
    }
}

impl<'a> From<&'a Bytes> for BulkString {
    fn from(val: &'a Bytes) -> BulkString {
        BulkString(val.clone())
    }
}

impl<'a> From<&'a ByteString> for BulkString {
    fn from(val: &'a ByteString) -> BulkString {
        BulkString(val.clone().into_bytes())
    }
}

impl<'a> From<&'a [u8]> for BulkString {
    fn from(val: &'a [u8]) -> BulkString {
        BulkString(Bytes::copy_from_slice(val))
    }
}

impl From<Vec<u8>> for BulkString {
    fn from(val: Vec<u8>) -> BulkString {
        BulkString(Bytes::from(val))
    }
}

/// A single RESP value, this owns the data that is to-be written to Redis.
///
/// Redis commands are sent as an [`Array`](Request::Array) of bulk strings,
/// see the [`array!`](crate::array) macro.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Request {
    /// Zero, one or more other `Request`s.
    Array(Vec<Request>),

    /// A bulk string, see [`BulkString`].
    BulkString(BulkString),

    /// A bulk string backed by static data, encoded without copying.
    BulkStatic(&'static [u8]),

    /// An integer, encoded as a bulk string of its decimal representation.
    BulkInteger(i64),

    /// A valid utf-8 string
    String(ByteString),

    /// Redis documentation defines an integer as being a signed 64-bit integer:
    /// <https://redis.io/topics/protocol#resp-integers>
    Integer(i64),
}

impl Request {
    /// Create bulk string request from a static str
    pub fn from_static(data: &'static str) -> Self {
        Request::BulkStatic(data.as_ref())
    }

    /// Create bulk string request from a static byte slice
    pub fn from_bstatic(data: &'static [u8]) -> Self {
        Request::BulkStatic(data)
    }

    /// Convenience function for building dynamic Redis commands with variable numbers of
    /// arguments, e.g. RPUSH
    ///
    /// `self` gets converted to an array if it is not an array.
    #[allow(clippy::should_implement_trait)]
    pub fn add<T>(mut self, other: T) -> Self
    where
        Request: From<T>,
    {
        match self {
            Request::Array(ref mut vals) => {
                vals.push(other.into());
                self
            }
            _ => Request::Array(vec![self, other.into()]),
        }
    }

    /// Convenience function for building dynamic Redis commands with variable numbers of
    /// arguments, e.g. RPUSH
    ///
    /// `self` gets converted to an array if it is not an array.
    pub fn extend<T>(mut self, other: impl IntoIterator<Item = T>) -> Self
    where
        Request: From<T>,
    {
        match self {
            Request::Array(ref mut vals) => {
                vals.extend(other.into_iter().map(|t| t.into()));
                self
            }
            _ => {
                let mut vals = vec![self];
                vals.extend(other.into_iter().map(|t| t.into()));
                Request::Array(vals)
            }
        }
    }
}

impl<T> From<T> for Request
where
    BulkString: From<T>,
{
    fn from(val: T) -> Request {
        Request::BulkString(val.into())
    }
}

impl From<i8> for Request {
    fn from(val: i8) -> Request {
        Request::Integer(val as i64)
    }
}

impl From<i16> for Request {
    fn from(val: i16) -> Request {
        Request::Integer(val as i64)
    }
}

impl From<i32> for Request {
    fn from(val: i32) -> Request {
        Request::Integer(val as i64)
    }
}

impl From<i64> for Request {
    fn from(val: i64) -> Request {
        Request::Integer(val)
    }
}

impl From<u8> for Request {
    fn from(val: u8) -> Request {
        Request::Integer(val as i64)
    }
}

impl From<u16> for Request {
    fn from(val: u16) -> Request {
        Request::Integer(val as i64)
    }
}

impl From<u32> for Request {
    fn from(val: u32) -> Request {
        Request::Integer(val as i64)
    }
}

impl From<usize> for Request {
    fn from(val: usize) -> Request {
        Request::Integer(val as i64)
    }
}

/// A single RESP value, this owns the data that is read from Redis.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Response {
    /// Null bulk string or null array, i.e. missing value
    Nil,

    /// Zero, one or more other `Response`s.
    Array(Vec<Response>),

    /// A bulk string. In Redis terminology a string is a byte-array, so this is stored as
    /// [`Bytes`] to allow clients to interpret the bytes as appropriate.
    Bytes(Bytes),

    /// A valid utf-8 string
    String(ByteString),

    /// An error from the Redis server
    Error(ByteString),

    /// Redis documentation defines an integer as being a signed 64-bit integer:
    /// <https://redis.io/topics/protocol#resp-integers>
    Integer(i64),
}

impl Response {
    /// Extract redis server error to Result
    pub fn into_result(self) -> Result<Response, ByteString> {
        match self {
            Response::Error(val) => Err(val),
            val => Ok(val),
        }
    }
}

impl TryFrom<Response> for Bytes {
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<Self, Self::Error> {
        if let Response::Bytes(bytes) = val {
            Ok(bytes)
        } else {
            Err(("Not a bytes object", val))
        }
    }
}

impl TryFrom<Response> for ByteString {
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<Self, Self::Error> {
        match val {
            Response::String(val) => Ok(val),
            Response::Bytes(val) => {
                if let Ok(val) = ByteString::try_from(val) {
                    Ok(val)
                } else {
                    Err(("Cannot convert into a string", Response::Nil))
                }
            }
            _ => Err(("Cannot convert into a string", val)),
        }
    }
}

impl TryFrom<Response> for i64 {
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<Self, Self::Error> {
        if let Response::Integer(i) = val {
            Ok(i)
        } else {
            Err(("Cannot be converted into an i64", val))
        }
    }
}

impl TryFrom<Response> for bool {
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<bool, Self::Error> {
        i64::try_from(val).and_then(|x| match x {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err((
                "i64 value cannot be represented as bool",
                Response::Integer(x),
            )),
        })
    }
}

impl<T> TryFrom<Response> for Vec<T>
where
    T: TryFrom<Response, Error = (&'static str, Response)>,
{
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<Vec<T>, Self::Error> {
        if let Response::Array(ary) = val {
            let mut ar = Vec::with_capacity(ary.len());
            for value in ary {
                ar.push(T::try_from(value)?);
            }
            Ok(ar)
        } else {
            Err(("Cannot be converted into a vector", val))
        }
    }
}

impl TryFrom<Response> for () {
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<(), Self::Error> {
        if let Response::String(string) = val {
            match string.as_ref() {
                "OK" => Ok(()),
                _ => Err(("Unexpected value within String", Response::String(string))),
            }
        } else {
            Err(("Unexpected value", val))
        }
    }
}

impl<A, B> TryFrom<Response> for (A, B)
where
    A: TryFrom<Response, Error = (&'static str, Response)>,
    B: TryFrom<Response, Error = (&'static str, Response)>,
{
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<(A, B), Self::Error> {
        match val {
            Response::Array(ary) => {
                if ary.len() == 2 {
                    let mut ary_iter = ary.into_iter();
                    Ok((
                        A::try_from(ary_iter.next().expect("No value"))?,
                        B::try_from(ary_iter.next().expect("No value"))?,
                    ))
                } else {
                    Err(("Array needs to be 2 elements", Response::Array(ary)))
                }
            }
            _ => Err(("Unexpected value", val)),
        }
    }
}

impl<A, B, C> TryFrom<Response> for (A, B, C)
where
    A: TryFrom<Response, Error = (&'static str, Response)>,
    B: TryFrom<Response, Error = (&'static str, Response)>,
    C: TryFrom<Response, Error = (&'static str, Response)>,
{
    type Error = (&'static str, Response);

    fn try_from(val: Response) -> Result<(A, B, C), Self::Error> {
        match val {
            Response::Array(ary) => {
                if ary.len() == 3 {
                    let mut ary_iter = ary.into_iter();
                    Ok((
                        A::try_from(ary_iter.next().expect("No value"))?,
                        B::try_from(ary_iter.next().expect("No value"))?,
                        C::try_from(ary_iter.next().expect("No value"))?,
                    ))
                } else {
                    Err(("Array needs to be 3 elements", Response::Array(ary)))
                }
            }
            _ => Err(("Unexpected value", val)),
        }
    }
}

macro_rules! impl_tryfrom_hashmap {
    ($map:ident) => {
        impl<K, T, S> TryFrom<Response> for $map<K, T, S>
        where
            K: TryFrom<Response, Error = (&'static str, Response)> + Hash + Eq,
            T: TryFrom<Response, Error = (&'static str, Response)>,
            S: BuildHasher + Default,
        {
            type Error = (&'static str, Response);

            fn try_from(val: Response) -> Result<$map<K, T, S>, Self::Error> {
                match val {
                    Response::Array(ary) => {
                        let mut map = $map::with_capacity_and_hasher(ary.len() / 2, S::default());
                        let mut items = ary.into_iter();

                        while let Some(k) = items.next() {
                            let key = K::try_from(k)?;
                            let value = T::try_from(items.next().ok_or((
                                "Cannot convert an odd number of elements into a hashmap",
                                Response::Nil,
                            ))?)?;
                            map.insert(key, value);
                        }

                        Ok(map)
                    }
                    _ => Err(("Cannot be converted into a hashmap", val)),
                }
            }
        }
    };
}

impl_tryfrom_hashmap!(HashMap);
impl_tryfrom_hashmap!(HashbrownMap);

macro_rules! impl_tryfrom_integers {
    ($($int_ty:ident),* $(,)*) => {
        $(
            #[allow(clippy::cast_lossless)]
            impl TryFrom<Response> for $int_ty {
                type Error = (&'static str, Response);

                fn try_from(val: Response) -> Result<Self, Self::Error> {
                    i64::try_from(val).and_then(|x| {
                        // $int_ty::MAX as i64 > 0 should be optimized out. It tests if
                        // the target integer type needs an "upper bounds" check
                        if x < ($int_ty::MIN as i64)
                            || ($int_ty::MAX as i64 > 0
                                && x > ($int_ty::MAX as i64))
                        {
                            Err((
                                concat!(
                                    "i64 value cannot be represented as {}",
                                    stringify!($int_ty),
                                ),
                                Response::Integer(x),
                            ))
                        } else {
                            Ok(x as $int_ty)
                        }
                    })
                }
            }
        )*
    };
}

impl_tryfrom_integers!(isize, usize, i32, u32, u64);

fn write_rn(buf: &mut BytePages) {
    buf.extend_from_slice(b"\r\n");
}

fn write_header(symb: u8, len: i64, buf: &mut BytePages) {
    let mut len_buf = itoa::Buffer::new();
    let rendered = len_buf.format(len).as_bytes();

    // symbol, at most 20 digits and CRLF
    let mut header = [0u8; 23];
    let end = rendered.len() + 3;
    header[0] = symb;
    header[1..end - 2].copy_from_slice(rendered);
    header[end - 2..end].copy_from_slice(b"\r\n");
    buf.extend_from_slice(&header[..end]);
}

fn write_string(symb: u8, string: &str, buf: &mut BytePages) {
    let bytes = string.as_bytes();
    buf.put_u8(symb);
    buf.extend_from_slice(bytes);
    write_rn(buf);
}

/// Finds the next `\r\n` starting at `idx`, returns position of `\r`
fn find_crlf(buf: &[u8], idx: usize) -> Option<usize> {
    let mut pos = idx;
    while let Some(p) = buf.get(pos..)?.iter().position(|b| *b == b'\r') {
        let p = pos + p;
        match buf.get(p + 1) {
            Some(b'\n') => return Some(p),
            Some(_) => pos = p + 1,
            None => return None,
        }
    }
    None
}

/// Parses `\r\n` terminated integer, returns position after `\r\n`
fn parse_int(buf: &[u8], idx: usize) -> Result<Option<(usize, i64)>, Error> {
    if let Some(p) = find_crlf(buf, idx) {
        let int_str = &buf[idx..p];
        match btoi::btoi(int_str) {
            Ok(int) => Ok(Some((p + 2, int))),
            Err(_) => Err(Error::Parse(format!(
                "Not an integer: {:?}",
                &int_str[..cmp::min(int_str.len(), 10)]
            ))),
        }
    } else {
        Ok(None)
    }
}

/// Incremental response scanner
///
/// Remembers the position of the first incomplete value, so the next
/// call resumes scanning instead of starting from the beginning of the buffer.
#[derive(Debug, Default)]
struct Scanner {
    pos: Cell<usize>,
    // number of values that are not scanned yet, 0 if no scan is in progress
    pending: Cell<usize>,
}

impl Scanner {
    /// Get the length of the first complete response in the buffer
    ///
    /// For an incomplete response, reserves read buffer capacity for the rest of it.
    fn frame_len(&self, buf: &mut BytesMut) -> Result<Option<usize>, Error> {
        let (pos, pending) = match self.pending.get() {
            // buffer can only grow while response is incomplete
            pending if pending > 0 && self.pos.get() <= buf.len() => (self.pos.get(), pending),
            _ => (0, 1),
        };
        match scan(buf, pos, pending) {
            Ok(Scan::Incomplete(pos, pending, need)) => {
                self.pos.set(pos);
                self.pending.set(pending);
                // io read buffer grows by small steps, reserve whole bulk string
                // in advance, size is capped as it comes from the server
                let len = buf.len();
                let target = if need > len {
                    cmp::min(need, cmp::max(len * 2, MAX_RESERVE))
                } else if len >= GROW_THRESHOLD {
                    len * 2
                } else {
                    0
                };
                if target > buf.capacity() {
                    buf.reserve(target - len);
                }
                Ok(None)
            }
            res => {
                self.pending.set(0);
                res.map(|scan| match scan {
                    Scan::Complete(len) => Some(len),
                    Scan::Incomplete(..) => None,
                })
            }
        }
    }
}

enum Scan {
    /// Length of the complete response
    Complete(usize),
    /// Start of the first incomplete value, number of values to scan
    /// and end of the incomplete bulk string (0 for other values)
    Incomplete(usize, usize, usize),
}

/// Scan response values starting at `pos`
///
/// Buffer is not modified, bulk string payloads are skipped by their length.
fn scan(buf: &[u8], mut pos: usize, mut pending: usize) -> Result<Scan, Error> {
    while pending > 0 {
        let start = pos;
        let incomplete = Ok(Scan::Incomplete(start, pending, 0));
        pending -= 1;

        let Some(&kind) = buf.get(pos) else {
            return incomplete;
        };
        match kind {
            b'+' | b'-' | b':' => match find_crlf(buf, pos + 1) {
                Some(p) => pos = p + 2,
                None => return incomplete,
            },
            b'$' => match parse_int(buf, pos + 1)? {
                Some((p, -1)) => pos = p,
                Some((p, size)) if size >= 0 => {
                    let end = usize::try_from(size)
                        .ok()
                        .and_then(|size| p.checked_add(size)?.checked_add(2))
                        .ok_or_else(|| Error::Parse(format!("Invalid string size: {size}")))?;
                    if end > buf.len() {
                        return Ok(Scan::Incomplete(start, pending + 1, end));
                    }
                    pos = end;
                }
                Some((_, size)) => {
                    return Err(Error::Parse(format!("Invalid string size: {size}")));
                }
                None => return incomplete,
            },
            b'*' => match parse_int(buf, pos + 1)? {
                Some((p, -1)) => pos = p,
                Some((p, size)) if size >= 0 => {
                    pending = usize::try_from(size)
                        .ok()
                        .and_then(|size| pending.checked_add(size))
                        .ok_or_else(|| Error::Parse(format!("Invalid array size: {size}")))?;
                    pos = p;
                }
                Some((_, size)) => {
                    return Err(Error::Parse(format!("Invalid array size: {size}")));
                }
                None => return incomplete,
            },
            b => return Err(Error::Parse(format!("Unexpected byte: {b}"))),
        }
    }
    Ok(Scan::Complete(pos))
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use ntex_bytes::{ByteString, Bytes, BytesMut};
    use ntex_codec::{Decoder, Encoder};
    use ntex_util::HashMap;

    use super::*;
    use crate::array;

    fn obj_to_bytes(obj: Request) -> Bytes {
        let mut pages = BytePages::default();
        Codec::new().encode(obj, &mut pages).unwrap();
        pages.freeze()
    }

    #[test]
    fn test_array_macro() {
        let resp_object = array!["SET", "x"];
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(bytes, b"*2\r\n$3\r\nSET\r\n$1\r\nx\r\n".as_ref());

        let resp_object = array!["RPUSH", "wyz"].extend(vec!["a", "b"]);
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(
            bytes,
            b"*4\r\n$5\r\nRPUSH\r\n$3\r\nwyz\r\n$1\r\na\r\n$1\r\nb\r\n".as_ref(),
        );

        let vals = vec!["a", "b"];
        let resp_object = array!["RPUSH", "xyz"].extend(&vals);
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(
            bytes,
            &b"*4\r\n$5\r\nRPUSH\r\n$3\r\nxyz\r\n$1\r\na\r\n$1\r\nb\r\n"[..],
        );
    }

    #[test]
    fn test_bulk_string() {
        // non-array request is sent as single element array
        let req_object = Request::BulkString(Bytes::from_static(b"THISISATEST").into());
        let bytes = obj_to_bytes(req_object);
        assert_eq!(bytes, b"*1\r\n$11\r\nTHISISATEST\r\n".as_ref());

        let mut bytes = BytesMut::copy_from_slice(b"$11\r\nTHISISATEST\r\n");
        let resp_object = Response::Bytes(Bytes::from_static(b"THISISATEST"));
        let deserialized = Codec::new().decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, resp_object);

        let bytes = obj_to_bytes(Request::from_static("PING"));
        assert_eq!(bytes, b"*1\r\n$4\r\nPING\r\n".as_ref());

        let bytes = obj_to_bytes(Request::Integer(i64::MIN));
        assert_eq!(bytes, b"*1\r\n:-9223372036854775808\r\n".as_ref());

        let bytes = obj_to_bytes(array!["INCRBY", "key"].add(Request::BulkInteger(-15)));
        assert_eq!(
            bytes,
            b"*3\r\n$6\r\nINCRBY\r\n$3\r\nkey\r\n$3\r\n-15\r\n".as_ref()
        );
    }

    #[test]
    fn test_large_bulk_string() {
        let data = Bytes::from(vec![b'x'; 64 * 1024]);
        let bytes = obj_to_bytes(array!["SET", "key", data.clone()]);
        let mut expected = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$65536\r\n".to_vec();
        expected.extend_from_slice(&data);
        expected.extend_from_slice(b"\r\n");
        assert_eq!(bytes, &expected[..]);

        let data = &*Vec::leak(vec![b'y'; 8 * 1024]);
        let bytes = obj_to_bytes(Request::BulkStatic(data));
        let mut expected = b"*1\r\n$8192\r\n".to_vec();
        expected.extend_from_slice(data);
        expected.extend_from_slice(b"\r\n");
        assert_eq!(bytes, &expected[..]);
    }

    #[test]
    fn test_array() {
        let req_object = Request::Array(vec![b"TEST1".as_ref().into(), b"TEST2".as_ref().into()]);
        let mut bytes = BytesMut::copy_from_slice(obj_to_bytes(req_object));
        let codec = Codec::new();
        assert_eq!(
            b"*2\r\n$5\r\nTEST1\r\n$5\r\nTEST2\r\n".to_vec(),
            bytes.to_vec()
        );

        let resp = Response::Array(vec![
            Response::Bytes(Bytes::from_static(b"TEST1")),
            Response::Bytes(Bytes::from_static(b"TEST2")),
        ]);
        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, resp);
    }

    #[test]
    fn test_decode_array() {
        let codec = Codec::new();

        let resp = Response::Array(vec![
            Response::Bytes(Bytes::from_static(b"TEST1")),
            Response::Bytes(Bytes::from_static(b"TEST2")),
        ]);

        let mut bytes = BytesMut::copy_from_slice(b"*2\r\n$5\r\nTEST1\r\n$5\r\nTEST2\r\n");
        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, resp);

        // uncomplete array data
        let mut bytes = BytesMut::copy_from_slice(b"*2\r\n$5\r\nTEST1\r\n");
        let result = codec.decode(&mut bytes).unwrap();
        assert!(result.is_none());

        // receiving remain data
        bytes.extend_from_slice(b"$5\r\nTEST2\r\n");

        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, resp);

        // $ echo -e "mget key1 key2\r\nquit" | curl -s telnet://localhost:6379 | python -c "import sys; print(repr(sys.stdin.read()))"
        // '*2\r\n$-1\r\n$-1\r\n+OK\r\n'
        let mut bytes = BytesMut::copy_from_slice(b"*2\r\n$-1\r\n$-1\r\n");
        let result = codec.decode(&mut bytes).unwrap();

        assert_eq!(
            result,
            Some(Response::Array(vec![Response::Nil, Response::Nil]))
        );

        // uncomplete nested array data
        // [[['a']]]
        // $ echo -e 'eval "return {{{\'a\'}}}" 0\r\nquit' | curl -s telnet://localhost:6379 | python -c "import sys; print(repr(sys.stdin.read()))"
        // '*1\r\n*1\r\n*1\r\n$1\r\na\r\n+OK\r\n'
        let mut bytes = BytesMut::copy_from_slice(b"*1\r\n*1\r\n*1\r\n");
        let result = codec.decode(&mut bytes).unwrap();
        assert_eq!(result, None);

        // receiving remain parts
        bytes.extend_from_slice(b"$1\r\na");
        let result = codec.decode(&mut bytes).unwrap();
        assert_eq!(result, None);

        bytes.extend_from_slice(b"\r");
        let result = codec.decode(&mut bytes).unwrap();
        assert_eq!(result, None);

        bytes.extend_from_slice(b"\n");
        let result = codec.decode(&mut bytes).unwrap();

        assert_eq!(
            result,
            Some(Response::Array(vec![Response::Array(vec![
                Response::Array(vec![Response::Bytes(Bytes::from_static(b"a"))])
            ])]))
        );
    }

    #[test]
    fn test_decode_partial() {
        let data: &[u8] =
            b"*2\r\n$1\r\na\r\n$5\r\nhe\r\n$\r\n*3\r\n:1\r\n*2\r\n+OK\r\n$-1\r\n-ERR e\r\n\
                            *0\r\n*-1\r\n$0\r\n\r\n";
        let expected = vec![
            Response::Array(vec![
                Response::Bytes(Bytes::from_static(b"a")),
                Response::Bytes(Bytes::from_static(b"he\r\n$")),
            ]),
            Response::Array(vec![
                Response::Integer(1),
                Response::Array(vec![
                    Response::String(ByteString::from_static("OK")),
                    Response::Nil,
                ]),
                Response::Error(ByteString::from_static("ERR e")),
            ]),
            Response::Array(vec![]),
            Response::Nil,
            Response::Bytes(Bytes::new()),
        ];

        // feed data in chunks of every size
        for chunk in 1..=data.len() {
            let codec = Codec::new();
            let mut buf = BytesMut::new();
            let mut items = Vec::new();
            for part in data.chunks(chunk) {
                buf.extend_from_slice(part);
                while let Some(item) = codec.decode(&mut buf).unwrap() {
                    items.push(item);
                }
            }
            assert!(buf.is_empty());
            assert_eq!(items, expected, "chunk size {chunk}");
        }
    }

    #[test]
    fn test_decode_errors() {
        for data in [
            &b"!1\r\n"[..],
            b"$x\r\n",
            b"$-2\r\n",
            b"*-2\r\n",
            b"$1\r\nab\r\n",
            b"+\xff\xfe\r\n",
            b":12a\r\n",
        ] {
            let mut buf = BytesMut::copy_from_slice(data);
            assert!(Codec::new().decode(&mut buf).is_err(), "{data:?}");
        }

        // huge array header does not allocate
        let mut buf = BytesMut::copy_from_slice(b"*9223372036854775807\r\n:1\r\n");
        assert_eq!(Codec::new().decode(&mut buf).unwrap(), None);
    }

    #[test]
    fn test_nil_string() {
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&b"$-1\r\n"[..]);

        let codec = Codec::new();
        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, Response::Nil);
    }

    #[test]
    fn test_integer_overflow() {
        let resp_object = Response::Integer(i64::MAX);
        let res = i32::try_from(resp_object);
        assert!(res.is_err());
    }

    #[test]
    fn test_integer_underflow() {
        let resp_object = Response::Integer(-2);
        let res = u64::try_from(resp_object);
        assert!(res.is_err());
    }

    #[test]
    fn test_integer_convesion() {
        let resp_object = Response::Integer(50);
        assert_eq!(u32::try_from(resp_object).unwrap(), 50);
    }

    #[test]
    fn test_hashmap_conversion() {
        let mut expected = HashMap::default();
        expected.insert(ByteString::from("KEY1"), ByteString::from("VALUE1"));
        expected.insert(ByteString::from("KEY2"), ByteString::from("VALUE2"));

        let resp_object = Response::Array(vec![
            Response::String(ByteString::from_static("KEY1")),
            Response::String(ByteString::from_static("VALUE1")),
            Response::String(ByteString::from_static("KEY2")),
            Response::String(ByteString::from_static("VALUE2")),
        ]);
        assert_eq!(
            HashMap::<ByteString, ByteString>::try_from(resp_object).unwrap(),
            expected
        );
    }

    #[test]
    fn test_hashmap_conversion_fails_with_odd_length_array() {
        let resp_object = Response::Array(vec![
            Response::String(ByteString::from_static("KEY1")),
            Response::String(ByteString::from_static("VALUE1")),
            Response::String(ByteString::from_static("KEY2")),
            Response::String(ByteString::from_static("VALUE2")),
            Response::String(ByteString::from_static("KEY3")),
        ]);
        let res = HashMap::<ByteString, ByteString>::try_from(resp_object);

        match res {
            Err((_, _)) => {}
            _ => panic!("Should not be able to convert an odd number of elements to a hashmap"),
        }
    }

    fn write_bytes(f: impl FnOnce(&mut Writer<'_>)) -> Bytes {
        let mut pages = BytePages::default();
        f(&mut Writer::new(&mut pages));
        pages.freeze()
    }

    #[test]
    fn test_writer_bulk_sizes() {
        for len in [0, 1, 9, 10, 63, 64, 65, 99, 100, 4096, 4097, 10_000] {
            let data = vec![b'x'; len];
            let expected = obj_to_bytes(Request::Array(vec![Request::BulkString(
                data.clone().into(),
            )]));

            assert_eq!(write_bytes(|w| w.args((&data,))), expected, "slice {len}");
            assert_eq!(
                write_bytes(|w| w.args((Bytes::from(data.clone()),))),
                expected,
                "bytes {len}"
            );
            assert_eq!(
                write_bytes(|w| w.args((BytesMut::from(&data[..]),))),
                expected,
                "bytes_mut {len}"
            );
        }
    }

    #[test]
    fn test_writer_args() {
        let s = String::from("str");
        let bs = ByteString::from_static("bstr");
        let b = Bytes::from_static(b"b");
        let bulk = BulkString::from("bulk");
        let bytes = write_bytes(|w| {
            w.args((
                "CMD",
                &s,
                s.clone(),
                b"arr",
                &b"sl"[..],
                &bs,
                bs.clone(),
                &b,
            ));
            w.args((
                b.clone(),
                vec![b'v'],
                &vec![b'w'],
                bulk.clone(),
                &bulk,
                &"r",
            ));
        });
        assert_eq!(
            bytes,
            b"*8\r\n$3\r\nCMD\r\n$3\r\nstr\r\n$3\r\nstr\r\n$3\r\narr\r\n$2\r\nsl\r\n\
              $4\r\nbstr\r\n$4\r\nbstr\r\n$1\r\nb\r\n\
              *6\r\n$1\r\nb\r\n$1\r\nv\r\n$1\r\nw\r\n$4\r\nbulk\r\n$4\r\nbulk\r\n$1\r\nr\r\n"
                .as_ref()
        );
    }

    #[test]
    fn test_writer_ints() {
        let bytes = write_bytes(|w| {
            w.array(6);
            w.arg(0u8);
            w.arg(-1i8);
            w.arg(i64::MIN);
            w.arg(u64::MAX);
            w.arg(1234usize);
            w.arg(-56isize);
        });
        assert_eq!(
            bytes,
            b"*6\r\n$1\r\n0\r\n$2\r\n-1\r\n$20\r\n-9223372036854775808\r\n\
              $20\r\n18446744073709551615\r\n$4\r\n1234\r\n$3\r\n-56\r\n"
                .as_ref()
        );
    }

    #[test]
    fn test_writer_request() {
        let req = array!["SET", "x"].add(Request::BulkInteger(10));
        assert_eq!(write_bytes(|w| w.request(req.clone())), obj_to_bytes(req));
        // non-array requests are sent as an array
        assert_eq!(
            write_bytes(|w| w.request(Request::from_static("PING"))),
            b"*1\r\n$4\r\nPING\r\n".as_ref()
        );
    }

    #[test]
    fn test_arg_list() {
        let mut args = ArgList::new("LPUSH");
        assert_eq!(args.name(), "LPUSH");
        args.push("key");
        args.push(vec![b'x'; 100]);
        args.push(-5i64);
        args.set_name("LPUSHX");

        let expected = obj_to_bytes(Request::Array(vec![
            Request::from_static("LPUSHX"),
            Request::from_static("key"),
            Request::BulkString(vec![b'x'; 100].into()),
            Request::BulkInteger(-5),
        ]));
        assert_eq!(write_bytes(|w| args.write(w)), expected);

        let empty = ArgList::new("UNSUBSCRIBE");
        assert_eq!(
            write_bytes(|w| empty.write(w)),
            b"*1\r\n$11\r\nUNSUBSCRIBE\r\n".as_ref()
        );
    }

    fn frame(data: &'static [u8]) -> Frame {
        Frame::new(Bytes::from_static(data))
    }

    #[test]
    fn test_frame_readers() {
        assert_eq!(frame(b":-12\r\n").int().unwrap(), -12);
        assert_eq!(
            frame(b"$3\r\nabc\r\n").bulk().unwrap(),
            Some(Bytes::from_static(b"abc"))
        );
        assert_eq!(frame(b"$0\r\n\r\n").bytes().unwrap(), Bytes::new());
        assert_eq!(frame(b"$-1\r\n").bulk().unwrap(), None);
        assert_eq!(frame(b"*-1\r\n").bulk().unwrap(), None);
        assert_eq!(frame(b"$-1\r\n").array().unwrap(), None);
        assert_eq!(frame(b"*-1\r\n").array().unwrap(), None);
        assert!(frame(b"$-1\r\n").is_nil());
        assert!(frame(b"*-1\r\n").is_nil());
        assert!(!frame(b"*0\r\n").is_nil());
        assert_eq!(frame(b"+OK\r\n").string().unwrap(), "OK");
        assert_eq!(frame(b"$2\r\nok\r\n").string().unwrap(), "ok");

        let mut f = frame(b"*3\r\n$1\r\na\r\n:1\r\n*2\r\n+x\r\n$-1\r\n");
        assert_eq!(f.array().unwrap(), Some(3));
        assert_eq!(f.bytes().unwrap(), Bytes::from_static(b"a"));
        assert_eq!(f.int().unwrap(), 1);
        assert_eq!(
            f.value().unwrap(),
            Response::Array(vec![Response::String("x".into()), Response::Nil])
        );
        assert!(matches!(f.int(), Err(CommandError::Protocol(_))));
    }

    #[test]
    fn test_frame_errors() {
        // redis errors
        let mut f = frame(b"*2\r\n-ERR one\r\n:1\r\n");
        assert_eq!(f.array().unwrap(), Some(2));
        assert!(matches!(f.int(), Err(CommandError::Error(e)) if e == "ERR one"));
        assert!(matches!(
            frame(b"-ERR two\r\n").decode::<crate::cmd::commands::KeysCommand>(),
            Err(CommandError::Error(e)) if e == "ERR two"
        ));
        assert_eq!(
            frame(b"-ERR\r\n").value().unwrap(),
            Response::Error("ERR".into())
        );

        // type mismatch returns the whole response
        let mut f = frame(b"*2\r\n:1\r\n:2\r\n");
        assert_eq!(f.array().unwrap(), Some(2));
        match f.bytes() {
            Err(CommandError::Output(_, val)) => {
                assert_eq!(
                    val,
                    Response::Array(vec![Response::Integer(1), Response::Integer(2)])
                );
            }
            res => panic!("{res:?}"),
        }
        assert!(matches!(
            frame(b"$-1\r\n").bytes(),
            Err(CommandError::Output(_, Response::Nil))
        ));
        assert!(matches!(
            frame(b"$1\r\n\xff\r\n").string(),
            Err(CommandError::Output(..))
        ));

        // protocol errors
        assert!(matches!(frame(b"").int(), Err(CommandError::Protocol(_))));
        assert!(matches!(frame(b":1").int(), Err(CommandError::Protocol(_))));
        assert!(matches!(
            frame(b"$3\r\nab\r\n").bulk(),
            Err(CommandError::Protocol(_))
        ));
        assert!(matches!(
            frame(b"$-2\r\n").bulk(),
            Err(CommandError::Protocol(_))
        ));
        assert!(matches!(
            frame(b"*100\r\n:1\r\n").array(),
            Err(CommandError::Protocol(_))
        ));
        assert!(matches!(
            frame(b"+\xff\r\n").string(),
            Err(CommandError::Protocol(_))
        ));
        assert!(matches!(
            frame(b"*1\r\n?\r\n").value(),
            Err(CommandError::Protocol(_))
        ));
    }

    #[test]
    fn test_frame_codec() {
        let codec = FrameCodec::default();
        let mut buf = BytesMut::from(&b"+OK\r\n*2\r\n$1\r\na\r\n:1"[..]);
        assert_eq!(
            codec.decode(&mut buf).unwrap().unwrap(),
            b"+OK\r\n".as_ref()
        );
        assert!(codec.decode(&mut buf).unwrap().is_none());
        buf.extend_from_slice(b"\r\n");
        assert_eq!(
            codec.decode(&mut buf).unwrap().unwrap(),
            b"*2\r\n$1\r\na\r\n:1\r\n".as_ref()
        );
        assert!(buf.is_empty());
    }

    #[test]
    fn test_decode_reserve() {
        let codec = Codec::new();
        // incomplete bulk string, capacity is reserved for the whole value
        let mut buf = BytesMut::from(&b"$100000\r\nabc"[..]);
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        assert!(buf.capacity() >= 100_011);
        buf.extend_from_slice(&[b'a'; 99_997]);
        buf.extend_from_slice(b"\r\n");
        let Response::Bytes(val) = codec.decode(&mut buf).unwrap().unwrap() else {
            panic!()
        };
        assert_eq!(val.len(), 100_000);

        // huge size, reservation is capped
        let codec = Codec::new();
        let mut buf = BytesMut::from(&b"*2\r\n$1000000000\r\nabc"[..]);
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        assert!(buf.capacity() >= MAX_RESERVE);
        assert!(buf.capacity() < 2 * MAX_RESERVE);

        // large incomplete array, buffer grows geometrically
        let codec = Codec::new();
        let mut buf = BytesMut::from(&b"*100000\r\n"[..]);
        while buf.len() < GROW_THRESHOLD {
            buf.extend_from_slice(b":1\r\n");
        }
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        assert!(buf.capacity() >= 2 * buf.len());
    }

    #[test]
    fn test_decode_resume() {
        let codec = Codec::new();
        let mut buf = BytesMut::from(&b"*3\r\n:1\r\n$5\r\nhe"[..]);
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        // scan resumes at the incomplete bulk string
        assert_eq!((codec.scan.pos.get(), codec.scan.pending.get()), (8, 2));
        buf.extend_from_slice(b"llo\r\n*1\r\n");
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        assert_eq!((codec.scan.pos.get(), codec.scan.pending.get()), (23, 1));
        buf.extend_from_slice(b"+OK\r\n:2\r\n");
        assert_eq!(
            codec.decode(&mut buf).unwrap().unwrap(),
            Response::Array(vec![
                Response::Integer(1),
                Response::Bytes(Bytes::from_static(b"hello")),
                Response::Array(vec![Response::String(ByteString::from_static("OK"))]),
            ])
        );
        assert_eq!(codec.scan.pending.get(), 0);
        // next response is scanned from the start of the buffer
        assert_eq!(
            codec.decode(&mut buf).unwrap().unwrap(),
            Response::Integer(2)
        );

        // error resets state
        let mut buf = BytesMut::from(&b"*2\r\n:1\r\n"[..]);
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        buf.extend_from_slice(b"!\r\n");
        assert!(codec.decode(&mut buf).is_err());
        let mut buf = BytesMut::from(&b":3\r\n"[..]);
        assert_eq!(
            codec.decode(&mut buf).unwrap().unwrap(),
            Response::Integer(3)
        );

        // buffer shorter than saved position resets state
        let mut buf = BytesMut::from(&b"*2\r\n:1\r\n"[..]);
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
        let mut buf = BytesMut::from(&b":4\r\n"[..]);
        assert_eq!(
            codec.decode(&mut buf).unwrap().unwrap(),
            Response::Integer(4)
        );
    }

    #[test]
    fn test_command_decode() {
        use crate::cmd::{SubscribeItem, TtlResult, commands::*};

        type SetCmd = SetCommand<&'static str, &'static str>;
        assert!(frame(b"+OK\r\n").decode::<SetCmd>().unwrap());
        assert!(!frame(b"$-1\r\n").decode::<SetCmd>().unwrap());
        assert!(matches!(
            frame(b"+QUEUED\r\n").decode::<SetCmd>(),
            Err(CommandError::Output(_, Response::String(_)))
        ));
        assert!(frame(b"+OK\r\n").decode::<SelectCommand>().unwrap());
        assert!(!frame(b":1\r\n").decode::<SelectCommand>().unwrap());
        assert_eq!(
            frame(b"$4\r\nPONG\r\n").decode::<PingCommand>().unwrap(),
            "PONG"
        );

        type BoolCmd = BoolOutputCommand<(&'static str,)>;
        assert!(frame(b":1\r\n").decode::<BoolCmd>().unwrap());
        assert!(matches!(
            frame(b":2\r\n").decode::<BoolCmd>(),
            Err(CommandError::Output(_, Response::Integer(2)))
        ));
        assert_eq!(
            frame(b":-1\r\n").decode::<TtlCommand<&str>>().unwrap(),
            TtlResult::NoExpire
        );
        assert_eq!(
            frame(b"*2\r\n$1\r\na\r\n+b\r\n")
                .decode::<KeysPatternCommand<&str>>()
                .unwrap(),
            vec![ByteString::from("a"), ByteString::from("b")]
        );

        let map = frame(b"*4\r\n$1\r\na\r\n$1\r\n1\r\n$1\r\nb\r\n$1\r\n2\r\n")
            .decode::<HGetAllCommand<&str>>()
            .unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[&Bytes::from_static(b"b")], Bytes::from_static(b"2"));
        assert!(matches!(
            frame(b"*1\r\n$1\r\na\r\n").decode::<HGetAllCommand<&str>>(),
            Err(CommandError::Output(..))
        ));

        assert_eq!(
            frame(b"*3\r\n$9\r\nsubscribe\r\n$2\r\nch\r\n:1\r\n")
                .decode::<SubscribeOutputCommand>()
                .unwrap(),
            SubscribeItem::Subscribed(Bytes::from_static(b"ch"))
        );
        assert_eq!(
            frame(b"*4\r\n$8\r\npmessage\r\n$2\r\nc*\r\n$2\r\nch\r\n$3\r\nmsg\r\n")
                .decode::<SubscribeOutputCommand>()
                .unwrap(),
            SubscribeItem::Message {
                pattern: Some(Bytes::from_static(b"c*")),
                channel: Bytes::from_static(b"ch"),
                payload: Bytes::from_static(b"msg"),
            }
        );
        assert!(matches!(
            frame(b"*3\r\n$7\r\nmessage\r\n$2\r\nch\r\n:1\r\n").decode::<SubscribeOutputCommand>(),
            Err(CommandError::Output(..))
        ));
        assert!(matches!(
            frame(b"*3\r\n$3\r\nfoo\r\n$2\r\nch\r\n:1\r\n").decode::<SubscribeOutputCommand>(),
            Err(CommandError::Output(..))
        ));
    }
}
