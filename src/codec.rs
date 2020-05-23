/*
 * Copyright 2017-2019 Ben Ashford
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */
//! Redis protocol codec

use std::collections::HashMap;
use std::convert::TryFrom;
use std::hash::{BuildHasher, Hash};
use std::{cmp, str};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use bytestring::ByteString;
use ntex_codec::{Decoder, Encoder};

use super::Error;

/// Codec to read/write redis values
pub struct Codec;

impl Encoder for Codec {
    type Item = Value;
    type Error = Error;

    fn encode(&mut self, msg: Value, buf: &mut BytesMut) -> Result<(), Self::Error> {
        match msg {
            Value::Nil => {
                write_header(b'$', -1, buf, 0);
            }
            Value::Array(ary) => {
                write_header(b'*', ary.len() as i64, buf, 0);
                for v in ary {
                    self.encode(v, buf)?;
                }
            }
            Value::Bytes(bstr) => {
                let len = bstr.len();
                write_header(b'$', len as i64, buf, len + 2);
                buf.extend(bstr);
                write_rn(buf);
            }
            Value::String(ref string) => {
                write_string(b'+', string, buf);
            }
            Value::Integer(val) => {
                // Simple integer are just the header
                write_header(b':', val, buf, 0);
            }
            Value::Error(ref string) => {
                write_string(b'-', string, buf);
            }
        }
        Ok(())
    }
}

impl Decoder for Codec {
    type Item = Value;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match decode(buf, 0)? {
            Some((pos, item)) => {
                buf.advance(pos);
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }
}

/// A single RESP value, this owns the data that is read/to-be written to Redis.
///
/// It is cloneable to allow multiple copies to be delivered in certain circumstances, e.g. multiple
/// subscribers to the same topic.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Nil,

    /// Zero, one or more other `Value`s.
    Array(Vec<Value>),

    /// A bulk string. In Redis terminology a string is a byte-array, so this is stored as a
    /// vector of `u8`s to allow clients to interpret the bytes as appropriate.
    Bytes(Bytes),

    /// A valid utf-8 string
    String(ByteString),

    /// An error from the Redis server
    Error(ByteString),

    /// Redis documentation defines an integer as being a signed 64-bit integer:
    /// https://redis.io/topics/protocol#resp-integers
    Integer(i64),
}

impl Value {
    /// Convenience function for building dynamic Redis commands with variable numbers of
    /// arguments, e.g. RPUSH
    ///
    /// Self get converted to array if it is not an array.
    pub fn append<T>(mut self, other: impl IntoIterator<Item = T>) -> Self
    where
        Value: From<T>,
    {
        match self {
            Value::Array(ref mut vals) => {
                vals.extend(other.into_iter().map(|t| t.into()));
                self
            }
            _ => {
                let mut vals = vec![self];
                vals.extend(other.into_iter().map(|t| t.into()));
                Value::Array(vals)
            }
        }
    }

    /// Extract redis server error to Result
    pub fn into_result(self) -> Result<Value, ByteString> {
        match self {
            Value::Error(val) => Err(val),
            val => Ok(val),
        }
    }
}

impl TryFrom<Value> for Bytes {
    type Error = Error;

    fn try_from(val: Value) -> Result<Self, Self::Error> {
        if let Value::Bytes(bytes) = val {
            Ok(bytes)
        } else {
            Err(Error::Decode("Not a bulk string", val))
        }
    }
}

impl TryFrom<Value> for ByteString {
    type Error = Error;

    fn try_from(val: Value) -> Result<Self, Self::Error> {
        if let Value::String(string) = val {
            Ok(string)
        } else {
            Err(Error::Decode("Cannot convert into a string", val))
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = Error;

    fn try_from(val: Value) -> Result<Self, Self::Error> {
        if let Value::Integer(i) = val {
            Ok(i)
        } else {
            Err(Error::Decode("Cannot be converted into an i64", val))
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = Error;

    fn try_from(val: Value) -> Result<bool, Self::Error> {
        i64::try_from(val).and_then(|x| match x {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Decode(
                "i64 value cannot be represented as bool",
                Value::Integer(x),
            )),
        })
    }
}

// impl<T: TryFrom<Value>> TryFrom<Value> for Option<T> {
//     type Error = Error;
//
//     fn try_from(val: Value) -> Result<Option<T>, Self::Error> {
//         match val {
//             Value::Nil => Ok(None),
//             x => Ok(Some(T::try_from(x)?)),
//         }
//     }
// }

impl<T> TryFrom<Value> for Vec<T>
where
    T: TryFrom<Value, Error = Error>,
{
    type Error = Error;

    fn try_from(val: Value) -> Result<Vec<T>, Self::Error> {
        if let Value::Array(ary) = val {
            let mut ar = Vec::with_capacity(ary.len());
            for value in ary {
                ar.push(T::try_from(value)?);
            }
            Ok(ar)
        } else {
            Err(Error::Decode("Cannot be converted into a vector", val))
        }
    }
}

impl TryFrom<Value> for () {
    type Error = Error;

    fn try_from(val: Value) -> Result<(), Self::Error> {
        if let Value::String(string) = val {
            match string.as_ref() {
                "OK" => Ok(()),
                _ => Err(Error::Decode(
                    "Unexpected value within String",
                    Value::String(string),
                )),
            }
        } else {
            Err(Error::Decode("Unexpected value", val))
        }
    }
}

impl<A, B> TryFrom<Value> for (A, B)
where
    A: TryFrom<Value, Error = Error>,
    B: TryFrom<Value, Error = Error>,
{
    type Error = Error;

    fn try_from(val: Value) -> Result<(A, B), Self::Error> {
        match val {
            Value::Array(ary) => {
                if ary.len() == 2 {
                    let mut ary_iter = ary.into_iter();
                    Ok((
                        A::try_from(ary_iter.next().expect("No value"))?,
                        B::try_from(ary_iter.next().expect("No value"))?,
                    ))
                } else {
                    Err(Error::Decode(
                        "Array needs to be 2 elements",
                        Value::Array(ary),
                    ))
                }
            }
            _ => Err(Error::Decode("Unexpected value", val)),
        }
    }
}

impl<A, B, C> TryFrom<Value> for (A, B, C)
where
    A: TryFrom<Value, Error = Error>,
    B: TryFrom<Value, Error = Error>,
    C: TryFrom<Value, Error = Error>,
{
    type Error = Error;

    fn try_from(val: Value) -> Result<(A, B, C), Self::Error> {
        match val {
            Value::Array(ary) => {
                if ary.len() == 3 {
                    let mut ary_iter = ary.into_iter();
                    Ok((
                        A::try_from(ary_iter.next().expect("No value"))?,
                        B::try_from(ary_iter.next().expect("No value"))?,
                        C::try_from(ary_iter.next().expect("No value"))?,
                    ))
                } else {
                    Err(Error::Decode(
                        "Array needs to be 3 elements",
                        Value::Array(ary),
                    ))
                }
            }
            _ => Err(Error::Decode("Unexpected value", val)),
        }
    }
}

impl<K, T, S> TryFrom<Value> for HashMap<K, T, S>
where
    K: TryFrom<Value, Error = Error> + Hash + Eq,
    T: TryFrom<Value, Error = Error>,
    S: BuildHasher + Default,
{
    type Error = Error;

    fn try_from(val: Value) -> Result<HashMap<K, T, S>, Self::Error> {
        match val {
            Value::Array(ary) => {
                let mut map = HashMap::with_capacity_and_hasher(ary.len(), S::default());
                let mut items = ary.into_iter();

                while let Some(k) = items.next() {
                    let key = K::try_from(k)?;
                    let value = T::try_from(items.next().ok_or_else(|| {
                        Error::Decode(
                            "Cannot convert an odd number of elements into a hashmap",
                            Value::Nil,
                        )
                    })?)?;

                    map.insert(key, value);
                }

                Ok(map)
            }
            _ => Err(Error::Decode("Cannot be converted into a hashmap", val)),
        }
    }
}

macro_rules! impl_fromresp_integers {
    ($($int_ty:ident),* $(,)*) => {
        $(
            #[allow(clippy::cast_lossless)]
            impl TryFrom<Value> for $int_ty {
                type Error = Error;

                fn try_from(val: Value) -> Result<Self, Self::Error> {
                    i64::try_from(val).and_then(|x| {
                        // $int_ty::max_value() as i64 > 0 should be optimized out. It tests if
                        // the target integer type needs an "upper bounds" check
                        if x < ($int_ty::min_value() as i64)
                            || ($int_ty::max_value() as i64 > 0
                                && x > ($int_ty::max_value() as i64))
                        {
                            Err(Error::Decode(
                                concat!(
                                    "i64 value cannot be represented as {}",
                                    stringify!($int_ty),
                                ),
                                Value::Integer(x),
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

impl_fromresp_integers!(isize, usize, i32, u32, u64);

impl From<ByteString> for Value {
    fn from(val: ByteString) -> Value {
        Value::String(val)
    }
}

impl From<String> for Value {
    fn from(val: String) -> Value {
        Value::String(ByteString::from(val))
    }
}

impl<'a> From<&'a String> for Value {
    fn from(val: &'a String) -> Value {
        Value::String(ByteString::from(val.as_str()))
    }
}

impl<'a> From<&'a str> for Value {
    fn from(val: &'a str) -> Value {
        Value::String(ByteString::from(val))
    }
}

impl From<Bytes> for Value {
    fn from(val: Bytes) -> Value {
        Value::Bytes(val)
    }
}

impl<'a> From<&'a Bytes> for Value {
    fn from(val: &'a Bytes) -> Value {
        Value::Bytes(val.clone())
    }
}

impl<'a> From<&'a [u8]> for Value {
    fn from(val: &'a [u8]) -> Value {
        Value::Bytes(Bytes::copy_from_slice(val))
    }
}

impl From<Vec<u8>> for Value {
    fn from(val: Vec<u8>) -> Value {
        Value::Bytes(Bytes::from(val))
    }
}

impl From<i8> for Value {
    fn from(val: i8) -> Value {
        Value::Integer(val as i64)
    }
}

impl From<i16> for Value {
    fn from(val: i16) -> Value {
        Value::Integer(val as i64)
    }
}

impl From<i32> for Value {
    fn from(val: i32) -> Value {
        Value::Integer(val as i64)
    }
}

impl From<i64> for Value {
    fn from(val: i64) -> Value {
        Value::Integer(val)
    }
}

impl From<u8> for Value {
    fn from(val: u8) -> Value {
        Value::Integer(val as i64)
    }
}

impl From<u16> for Value {
    fn from(val: u16) -> Value {
        Value::Integer(val as i64)
    }
}

impl From<u32> for Value {
    fn from(val: u32) -> Value {
        Value::Integer(val as i64)
    }
}

impl From<usize> for Value {
    fn from(val: usize) -> Value {
        Value::Integer(val as i64)
    }
}

fn write_rn(buf: &mut BytesMut) {
    buf.extend_from_slice(b"\r\n");
}

fn write_header(symb: u8, len: i64, buf: &mut BytesMut, body_size: usize) {
    let mut len_buf = [0; 32];
    let size = itoa::write(&mut len_buf[..], len).unwrap();
    buf.reserve(3 + size + body_size);
    buf.put_u8(symb);
    buf.extend_from_slice(&len_buf[..size]);
    write_rn(buf);
}

fn write_string(symb: u8, string: &str, buf: &mut BytesMut) {
    let bytes = string.as_bytes();
    buf.reserve(3 + bytes.len());
    buf.put_u8(symb);
    buf.extend_from_slice(bytes);
    write_rn(buf);
}

type DecodeResult = Result<Option<(usize, Value)>, Error>;

fn decode(buf: &mut BytesMut, idx: usize) -> DecodeResult {
    match buf[idx] {
        b'$' => decode_bytes(buf, idx + 1),
        b'*' => decode_array(buf, idx + 1),
        b':' => decode_integer(buf, idx + 1),
        b'+' => decode_string(buf, idx + 1),
        b'-' => decode_error(buf, idx + 1),
        _ => Err(Error::Parse(format!("Unexpected byte: {}", buf[idx]))),
    }
}

fn decode_raw_integer(buf: &mut BytesMut, idx: usize) -> Result<Option<(usize, i64)>, Error> {
    // Many RESP types have their length (which is either bytes or "number of elements", depending on context)
    // encoded as a string, terminated by "\r\n", this looks for them.
    let (pos, int_str) = if let Some(pos) = buf[idx..].windows(2).position(|w| w == b"\r\n") {
        (idx + pos + 2, &buf[idx..idx + pos])
    } else {
        return Ok(None);
    };

    // Redis integers are transmitted as strings, so we first convert the raw bytes into a string...
    match btoi::btoi(int_str) {
        Ok(int) => Ok(Some((pos, int))),
        Err(_) => Err(Error::Parse(format!(
            "Not an integer: {:?}",
            &int_str[..cmp::min(int_str.len(), 10)]
        ))),
    }
}

fn decode_bytes(buf: &mut BytesMut, idx: usize) -> DecodeResult {
    match decode_raw_integer(buf, idx)? {
        Some((pos, -1)) => Ok(Some((pos, Value::Nil))),
        Some((pos, size)) if size >= 0 => {
            let size = size as usize;
            let remaining = buf.len() - pos;
            let required_bytes = size + 2;

            if remaining < required_bytes {
                return Ok(None);
            }
            buf.advance(pos);
            Ok(Some((2, Value::Bytes(buf.split_to(size).freeze()))))
        }
        Some((_, size)) => Err(Error::Parse(format!("Invalid string size: {}", size))),
        None => Ok(None),
    }
}

fn decode_array(buf: &mut BytesMut, idx: usize) -> DecodeResult {
    match decode_raw_integer(buf, idx)? {
        Some((pos, -1)) => Ok(Some((pos, Value::Nil))),
        Some((pos, size)) if size >= 0 => {
            let size = size as usize;
            let mut pos = pos;
            let mut values = Vec::with_capacity(size);
            for _ in 0..size {
                match decode(buf, pos) {
                    Ok(None) => return Ok(None),
                    Ok(Some((new_pos, value))) => {
                        values.push(value);
                        pos = new_pos;
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(Some((pos, Value::Array(values))))
        }
        Some((_, size)) => Err(Error::Parse(format!("Invalid array size: {}", size))),
        None => Ok(None),
    }
}

fn decode_integer(buf: &mut BytesMut, idx: usize) -> DecodeResult {
    if let Some((pos, int)) = decode_raw_integer(buf, idx)? {
        Ok(Some((pos, Value::Integer(int))))
    } else {
        Ok(None)
    }
}

/// A simple string is any series of bytes that ends with `\r\n`
fn decode_string(buf: &mut BytesMut, idx: usize) -> DecodeResult {
    if let Some((pos, string)) = scan_string(buf, idx)? {
        Ok(Some((pos, Value::String(string))))
    } else {
        Ok(None)
    }
}

fn decode_error(buf: &mut BytesMut, idx: usize) -> DecodeResult {
    if let Some((pos, string)) = scan_string(buf, idx)? {
        Ok(Some((pos, Value::Error(string))))
    } else {
        Ok(None)
    }
}

fn scan_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(usize, ByteString)>, Error> {
    if let Some(pos) = buf[idx..].windows(2).position(|w| w == b"\r\n") {
        buf.advance(idx);
        match ByteString::try_from(buf.split_to(pos)) {
            Ok(s) => Ok(Some((2, s))),
            Err(_) => Err(Error::Parse(format!(
                "Not a valid string: {:?}",
                &buf[idx..idx + cmp::min(pos, 10)]
            ))),
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::convert::TryFrom;

    use bytes::{Bytes, BytesMut};
    use bytestring::ByteString;
    use ntex_codec::{Decoder, Encoder};

    use crate::{array, Codec, Error, Value};

    fn obj_to_bytes(obj: Value) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        let mut codec = Codec;
        codec.encode(obj, &mut bytes).unwrap();
        bytes.to_vec()
    }

    #[test]
    fn test_array_macro() {
        let resp_object = array!["SET", "x"];
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(b"*2\r\n+SET\r\n+x\r\n", bytes.as_slice());

        let resp_object = array![b"SET".as_ref(), b"x".as_ref()];
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(b"*2\r\n$3\r\nSET\r\n$1\r\nx\r\n", bytes.as_slice());

        let resp_object =
            array![b"RPUSH".as_ref(), b"wyz".as_ref()].append(vec![b"a".as_ref(), b"b".as_ref()]);
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(
            b"*4\r\n$5\r\nRPUSH\r\n$3\r\nwyz\r\n$1\r\na\r\n$1\r\nb\r\n".as_ref(),
            bytes.as_slice()
        );

        let vals = vec![Bytes::from_static(b"a"), Bytes::from_static(b"b")];
        let resp_object = array![b"RPUSH".as_ref(), b"xyz".as_ref()].append(&vals);
        let bytes = obj_to_bytes(resp_object);
        assert_eq!(
            &b"*4\r\n$5\r\nRPUSH\r\n$3\r\nxyz\r\n$1\r\na\r\n$1\r\nb\r\n"[..],
            bytes.as_slice()
        );
    }

    #[test]
    fn test_bulk_string() {
        let resp_object = Value::Bytes(Bytes::from_static(b"THISISATEST"));
        let mut bytes = BytesMut::new();
        let mut codec = Codec;
        codec.encode(resp_object.clone(), &mut bytes).unwrap();
        assert_eq!(b"$11\r\nTHISISATEST\r\n".to_vec(), bytes.to_vec());

        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, resp_object);
    }

    #[test]
    fn test_array() {
        let resp_object = Value::Array(vec![b"TEST1".as_ref().into(), b"TEST2".as_ref().into()]);
        let mut bytes = BytesMut::new();
        let mut codec = Codec;
        codec.encode(resp_object.clone(), &mut bytes).unwrap();
        assert_eq!(
            b"*2\r\n$5\r\nTEST1\r\n$5\r\nTEST2\r\n".to_vec(),
            bytes.to_vec()
        );

        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, resp_object);
    }

    #[test]
    fn test_nil_string() {
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&b"$-1\r\n"[..]);

        let mut codec = Codec;
        let deserialized = codec.decode(&mut bytes).unwrap().unwrap();
        assert_eq!(deserialized, Value::Nil);
    }

    #[test]
    fn test_integer_overflow() {
        let resp_object = Value::Integer(i64::max_value());
        let res = i32::try_from(resp_object);
        assert!(res.is_err());
    }

    #[test]
    fn test_integer_underflow() {
        let resp_object = Value::Integer(-2);
        let res = u64::try_from(resp_object);
        assert!(res.is_err());
    }

    #[test]
    fn test_integer_convesion() {
        let resp_object = Value::Integer(50);
        assert_eq!(u32::try_from(resp_object).unwrap(), 50);
    }

    #[test]
    fn test_hashmap_conversion() {
        let mut expected = HashMap::new();
        expected.insert(
            ByteString::from("KEY1").into(),
            ByteString::from("VALUE1").into(),
        );
        expected.insert(
            ByteString::from("KEY2").into(),
            ByteString::from("VALUE2").into(),
        );

        let resp_object = Value::Array(vec![
            "KEY1".into(),
            "VALUE1".into(),
            "KEY2".into(),
            "VALUE2".into(),
        ]);
        assert_eq!(
            HashMap::<ByteString, ByteString>::try_from(resp_object).unwrap(),
            expected
        );
    }

    #[test]
    fn test_hashmap_conversion_fails_with_odd_length_array() {
        let resp_object = Value::Array(vec![
            "KEY1".into(),
            "VALUE1".into(),
            "KEY2".into(),
            "VALUE2".into(),
            "KEY3".into(),
        ]);
        let res = HashMap::<ByteString, ByteString>::try_from(resp_object);

        match res {
            Err(Error::Decode(_, _)) => {}
            _ => panic!("Should not be able to convert an odd number of elements to a hashmap"),
        }
    }
}
