// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use serde::{ser, Serialize};

/// Serialize the given data structure as a `Vec<u8>` of BCS.
///
/// # Errors
///
/// Returns an error if:
/// - `T`'s implementation of [`Serialize`] decides to fail
/// - `T` contains sequences longer than [`MAX_SEQUENCE_LENGTH`](crate::MAX_SEQUENCE_LENGTH)
/// - `T` attempts to serialize an unsupported datatype (f32, f64, or char)
/// - The container depth exceeds [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH)
/// - An I/O error occurs while writing
///
/// # Examples
///
/// ```
/// use bcs::to_bytes;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Ip([u8; 4]);
///
/// #[derive(Serialize)]
/// struct Port(u16);
///
/// #[derive(Serialize)]
/// struct Service {
///     ip: Ip,
///     port: Vec<Port>,
///     connection_max: Option<u32>,
///     enabled: bool,
/// }
///
/// let service = Service {
///     ip: Ip([192, 168, 1, 1]),
///     port: vec![Port(8001), Port(8002), Port(8003)],
///     connection_max: Some(5000),
///     enabled: false,
/// };
///
/// let bytes = to_bytes(&service).unwrap();
/// let expected = vec![
///     0xc0, 0xa8, 0x01, 0x01, 0x03, 0x41, 0x1f, 0x42,
///     0x1f, 0x43, 0x1f, 0x01, 0x88, 0x13, 0x00, 0x00,
///     0x00,
/// ];
/// assert_eq!(bytes, expected);
/// ```
pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut output = Vec::new();
    serialize_into(&mut output, value)?;
    Ok(output)
}

/// Same as [`to_bytes`] but pre-allocates the output buffer with the given capacity.
///
/// This can improve performance when you have a good estimate of the serialized size,
/// as it avoids reallocations during serialization.
///
/// # Errors
///
/// Returns an error if any error condition from [`to_bytes`] occurs.
///
/// # Examples
///
/// ```
/// use bcs::to_bytes_with_capacity;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Data {
///     values: Vec<u64>,
/// }
///
/// let data = Data { values: vec![1, 2, 3, 4, 5] };
/// // Pre-allocate for length prefix (1 byte) + 5 u64s (40 bytes)
/// let bytes = to_bytes_with_capacity(&data, 41).unwrap();
/// ```
pub fn to_bytes_with_capacity<T>(value: &T, capacity: usize) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut output = Vec::with_capacity(capacity);
    serialize_into(&mut output, value)?;
    Ok(output)
}

/// Same as [`to_bytes`] but use `limit` as max container depth instead of
/// [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH).
///
/// # Errors
///
/// Returns an error if:
/// - `limit` exceeds [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH)
/// - Any error condition from [`to_bytes`] occurs
pub fn to_bytes_with_limit<T>(value: &T, limit: usize) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    if limit > crate::MAX_CONTAINER_DEPTH {
        return Err(Error::NotSupported(
            "limit exceeds the max allowed depth 500",
        ));
    }
    let mut output = Vec::new();
    serialize_into_with_limit(&mut output, value, limit)?;
    Ok(output)
}

/// Same as [`to_bytes`] but write directly into an [`std::io::Write`] object.
///
/// # Errors
///
/// Returns an error if any error condition from [`to_bytes`] occurs.
pub fn serialize_into<W, T>(write: &mut W, value: &T) -> Result<()>
where
    W: ?Sized + std::io::Write,
    T: ?Sized + Serialize,
{
    let serializer = Serializer::new(write, crate::MAX_CONTAINER_DEPTH);
    value.serialize(serializer)
}

/// Same as [`serialize_into`] but use `limit` as max container depth instead of
/// [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH).
///
/// # Errors
///
/// Returns an error if:
/// - `limit` exceeds [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH)
/// - Any error condition from [`serialize_into`] occurs
pub fn serialize_into_with_limit<W, T>(write: &mut W, value: &T, limit: usize) -> Result<()>
where
    W: ?Sized + std::io::Write,
    T: ?Sized + Serialize,
{
    if limit > crate::MAX_CONTAINER_DEPTH {
        return Err(Error::NotSupported(
            "limit exceeds the max allowed depth 500",
        ));
    }
    let serializer = Serializer::new(write, limit);
    value.serialize(serializer)
}

struct WriteCounter(usize);

impl std::io::Write for WriteCounter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = buf.len();
        self.0 = self
            .0
            .checked_add(len)
            .ok_or_else(|| std::io::Error::other("WriteCounter reached max value"))?;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Same as [`to_bytes`] but only return the size of the serialized bytes.
///
/// This is useful for pre-allocating buffers or validating size constraints
/// without actually performing the serialization.
///
/// # Errors
///
/// Returns an error if any error condition from [`to_bytes`] occurs.
pub fn serialized_size<T>(value: &T) -> Result<usize>
where
    T: ?Sized + Serialize,
{
    let mut counter = WriteCounter(0);
    serialize_into(&mut counter, value)?;
    Ok(counter.0)
}

/// Same as [`serialized_size`] but use `limit` as max container depth instead of
/// [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH).
///
/// # Errors
///
/// Returns an error if:
/// - `limit` exceeds [`MAX_CONTAINER_DEPTH`](crate::MAX_CONTAINER_DEPTH)
/// - Any error condition from [`serialized_size`] occurs
pub fn serialized_size_with_limit<T>(value: &T, limit: usize) -> Result<usize>
where
    T: ?Sized + Serialize,
{
    if limit > crate::MAX_CONTAINER_DEPTH {
        return Err(Error::NotSupported(
            "limit exceeds the max allowed depth 500",
        ));
    }
    let mut counter = WriteCounter(0);
    serialize_into_with_limit(&mut counter, value, limit)?;
    Ok(counter.0)
}

/// Returns whether BCS is a human-readable format.
///
/// This always returns `false` as BCS is a binary format.
#[must_use]
pub fn is_human_readable() -> bool {
    let mut output = Vec::new();
    let serializer = Serializer::new(&mut output, crate::MAX_CONTAINER_DEPTH);
    ser::Serializer::is_human_readable(&serializer)
}

/// Serialization implementation for BCS
struct Serializer<'a, W: ?Sized> {
    output: &'a mut W,
    max_remaining_depth: usize,
}

impl<'a, W> Serializer<'a, W>
where
    W: ?Sized + std::io::Write,
{
    /// Creates a new `Serializer` which will emit BCS.
    #[inline]
    fn new(output: &'a mut W, max_remaining_depth: usize) -> Self {
        Self {
            output,
            max_remaining_depth,
        }
    }

    /// Encode a u32 as ULEB128. Optimized for common small values.
    #[inline]
    fn output_u32_as_uleb128(&mut self, value: u32) -> Result<()> {
        // Fast path: single byte (values 0-127) - very common for lengths and variant indices
        if value < 0x80 {
            self.output.write_all(&[value as u8])?;
            return Ok(());
        }

        // Multi-byte encoding - pre-compute all bytes to minimize write calls
        // Max ULEB128 encoding for u32 is 5 bytes
        let mut buf = [0u8; 5];
        let mut i = 0;
        let mut v = value;

        while v >= 0x80 {
            buf[i] = (v as u8 & 0x7f) | 0x80;
            v >>= 7;
            i += 1;
        }
        buf[i] = v as u8;

        self.output.write_all(&buf[..=i])?;
        Ok(())
    }

    #[inline]
    fn output_variant_index(&mut self, v: u32) -> Result<()> {
        self.output_u32_as_uleb128(v)
    }

    /// Serialize a sequence length as a u32.
    #[inline]
    fn output_seq_len(&mut self, len: usize) -> Result<()> {
        if len > crate::MAX_SEQUENCE_LENGTH {
            return Err(Error::ExceededMaxLen(len));
        }
        self.output_u32_as_uleb128(len as u32)
    }

    #[inline]
    fn enter_named_container(&mut self, name: &'static str) -> Result<()> {
        if self.max_remaining_depth == 0 {
            return Err(Error::ExceededContainerDepthLimit(name));
        }
        self.max_remaining_depth -= 1;
        Ok(())
    }
}

impl<'a, W> ser::Serializer for Serializer<'a, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.write_all(&[u8::from(v)])?;
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.output.write_all(&[v as u8])?;
        Ok(())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.output.write_all(&(v as u16).to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.output.write_all(&(v as u32).to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output.write_all(&(v as u64).to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.output.write_all(&(v as u128).to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.write_all(&[v])?;
        Ok(())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    #[inline]
    fn serialize_u128(self, v: u128) -> Result<()> {
        self.output.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(Error::NotSupported("serialize_f32"))
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(Error::NotSupported("serialize_f64"))
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::NotSupported("serialize_char"))
    }

    // Just serialize the string as a raw byte array
    #[inline]
    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }

    // Serialize a byte array as an array of bytes.
    #[inline]
    fn serialize_bytes(mut self, v: &[u8]) -> Result<()> {
        self.output_seq_len(v.len())?;
        self.output.write_all(v)?;
        Ok(())
    }

    // An absent optional is represented as `00`
    #[inline]
    fn serialize_none(self) -> Result<()> {
        self.output.write_all(&[0])?;
        Ok(())
    }

    // A present optional is represented as `01` followed by the serialized value
    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output.write_all(&[1])?;
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(mut self, name: &'static str) -> Result<()> {
        self.enter_named_container(name)?;
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)
    }

    #[inline]
    fn serialize_newtype_struct<T>(mut self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.enter_named_container(name)?;
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)?;
        value.serialize(self)
    }

    // The start of the sequence, each value, and the end are three separate
    // method calls. This one is responsible only for serializing the start,
    // which for BCS is either nothing for fixed structures or for variable
    // length structures, the length encoded as a u32.
    #[inline]
    fn serialize_seq(mut self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        if let Some(len) = len {
            self.output_seq_len(len)?;
            Ok(self)
        } else {
            Err(Error::MissingLen)
        }
    }

    // Tuples are fixed sized structs so we don't need to encode the length
    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_struct(
        mut self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.enter_named_container(name)?;
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)?;
        Ok(self)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(self))
    }

    #[inline]
    fn serialize_struct(
        mut self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.enter_named_container(name)?;
        Ok(self)
    }

    #[inline]
    fn serialize_struct_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.enter_named_container(name)?;
        self.output_variant_index(variant_index)?;
        Ok(self)
    }

    // BCS is not a human readable format
    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<W> ser::SerializeSeq for Serializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W> ser::SerializeTuple for Serializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W> ser::SerializeTupleStruct for Serializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W> ser::SerializeTupleVariant for Serializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

#[doc(hidden)]
struct MapSerializer<'a, W: ?Sized> {
    serializer: Serializer<'a, W>,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    next_key: Option<Vec<u8>>,
}

impl<'a, W: ?Sized> MapSerializer<'a, W> {
    #[inline]
    fn new(serializer: Serializer<'a, W>) -> Self {
        MapSerializer {
            serializer,
            entries: Vec::new(),
            next_key: None,
        }
    }
}

impl<W> ser::SerializeMap for MapSerializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.next_key.is_some() {
            return Err(Error::ExpectedMapValue);
        }

        let mut output = Vec::new();
        key.serialize(Serializer::new(
            &mut output,
            self.serializer.max_remaining_depth,
        ))?;
        self.next_key = Some(output);
        Ok(())
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self.next_key.take() {
            Some(key) => {
                let mut output = Vec::new();
                value.serialize(Serializer::new(
                    &mut output,
                    self.serializer.max_remaining_depth,
                ))?;
                self.entries.push((key, output));
                Ok(())
            }
            None => Err(Error::ExpectedMapKey),
        }
    }

    fn end(mut self) -> Result<()> {
        if self.next_key.is_some() {
            return Err(Error::ExpectedMapValue);
        }
        self.entries.sort_unstable_by(|e1, e2| e1.0.cmp(&e2.0));

        // Security: Detect and reject duplicate keys instead of silently dropping them.
        // This ensures serialization is lossless and maintains data integrity.
        for i in 1..self.entries.len() {
            if self.entries[i].0 == self.entries[i - 1].0 {
                return Err(Error::NonCanonicalMap);
            }
        }

        let len = self.entries.len();
        self.serializer.output_seq_len(len)?;

        for (key, value) in &self.entries {
            self.serializer.output.write_all(key)?;
            self.serializer.output.write_all(value)?;
        }

        Ok(())
    }
}

impl<W> ser::SerializeStruct for Serializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W> ser::SerializeStructVariant for Serializer<'_, W>
where
    W: ?Sized + std::io::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Serializer::new(self.output, self.max_remaining_depth))
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}
