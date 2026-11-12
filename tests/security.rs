// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Security-focused regression tests.
//!
//! Threat model: the bytes passed to `from_bytes` (and friends) are
//! attacker-controlled, while the target type is chosen by the application.
//! For arbitrary, truncated, or mutated input the deserializer must never
//! panic, never allocate memory disproportionate to the input it was given,
//! and must reject every non-canonical encoding. The serializer must never
//! silently drop data.

#![allow(non_local_definitions)]
// Mirrors the allowance in tests/serde.rs: roundtrip helpers take values.
#![allow(clippy::needless_pass_by_value)]

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    marker::PhantomData,
};

use proptest::prelude::*;
use serde::{
    de::{self, DeserializeOwned, Visitor},
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Serialize, Serializer,
};

use bcs::{
    from_bytes, from_bytes_seed, from_bytes_seed_with_limit, from_bytes_with_limit,
    is_human_readable, serialize_into, serialize_into_with_limit, serialized_size,
    serialized_size_with_limit, to_bytes, to_bytes_with_limit, Error, MAX_CONTAINER_DEPTH,
    MAX_SEQUENCE_LENGTH,
};

fn roundtrip<T>(value: T)
where
    T: Serialize + DeserializeOwned + fmt::Debug + PartialEq,
{
    let bytes = to_bytes(&value).unwrap();
    assert_eq!(serialized_size(&value).unwrap(), bytes.len());
    assert_eq!(from_bytes::<T>(&bytes).unwrap(), value);
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum Kind {
    A,
    B(u16),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Record {
    id: u32,
    label: String,
    payload: Vec<u8>,
    flag: bool,
    kind: Kind,
}

fn sample_record() -> Record {
    Record {
        id: 0x1234_5678,
        label: "diem".to_string(),
        payload: vec![0xC0, 0xDE],
        flag: true,
        kind: Kind::B(8000),
    }
}

// ---------------------------------------------------------------------------
// Allocation safety: untrusted length prefixes
// ---------------------------------------------------------------------------

/// A sequence visitor that records the `size_hint` it is offered and then
/// consumes elements normally. Downstream `Deserialize` impls may use this
/// hint to pre-allocate, so it must be bounded by the actual input size.
#[derive(Debug, PartialEq)]
struct ObservedSeq {
    hint: Option<usize>,
    elements: Vec<u8>,
}

impl<'de> Deserialize<'de> for ObservedSeq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct SeqVisitor;
        impl<'de> Visitor<'de> for SeqVisitor {
            type Value = ObservedSeq;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<ObservedSeq, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let hint = seq.size_hint();
                let mut elements = Vec::new();
                while let Some(e) = seq.next_element::<u8>()? {
                    elements.push(e);
                }
                Ok(ObservedSeq { hint, elements })
            }
        }
        deserializer.deserialize_seq(SeqVisitor)
    }
}

#[derive(Debug, PartialEq)]
struct ObservedMap {
    hint: Option<usize>,
    entries: Vec<(u8, u8)>,
}

impl<'de> Deserialize<'de> for ObservedMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct MapVisitor;
        impl<'de> Visitor<'de> for MapVisitor {
            type Value = ObservedMap;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a map")
            }

            fn visit_map<A>(self, mut access: A) -> Result<ObservedMap, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let hint = access.size_hint();
                let mut entries = Vec::new();
                while let Some(e) = access.next_entry::<u8, u8>()? {
                    entries.push(e);
                }
                Ok(ObservedMap { hint, entries })
            }
        }
        deserializer.deserialize_map(MapVisitor)
    }
}

/// Records the offered hint without consuming any elements, so a header
/// claiming a huge length with no payload can be inspected directly.
#[derive(Debug, PartialEq)]
struct SeqHint(Option<usize>);

impl<'de> Deserialize<'de> for SeqHint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct SeqHintVisitor;
        impl<'de> Visitor<'de> for SeqHintVisitor {
            type Value = SeqHint;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a sequence")
            }
            fn visit_seq<A>(self, seq: A) -> Result<SeqHint, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Ok(SeqHint(seq.size_hint()))
            }
        }
        deserializer.deserialize_seq(SeqHintVisitor)
    }
}

#[derive(Debug, PartialEq)]
struct MapHint(Option<usize>);

impl<'de> Deserialize<'de> for MapHint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct MapHintVisitor;
        impl<'de> Visitor<'de> for MapHintVisitor {
            type Value = MapHint;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a map")
            }
            fn visit_map<A>(self, access: A) -> Result<MapHint, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                Ok(MapHint(access.size_hint()))
            }
        }
        deserializer.deserialize_map(MapHintVisitor)
    }
}

#[test]
fn seq_size_hint_is_bounded_by_remaining_input() {
    // Header claims MAX_SEQUENCE_LENGTH elements; zero payload bytes remain.
    // Before the fix this reported Some(2_147_483_647) to the visitor.
    assert_eq!(
        from_bytes::<SeqHint>(&[0xff, 0xff, 0xff, 0xff, 0x07]),
        Ok(SeqHint(Some(0)))
    );
    // A visitor that does consume elements sees a clean EOF, not a panic,
    // when the claimed length exceeds the available bytes.
    assert_eq!(from_bytes::<ObservedSeq>(&[10, 1, 2, 3]), Err(Error::Eof));

    // A well-formed sequence still reports its exact length.
    let observed: ObservedSeq = from_bytes(&[3, 9, 8, 7]).unwrap();
    assert_eq!(
        observed,
        ObservedSeq {
            hint: Some(3),
            elements: vec![9, 8, 7],
        }
    );
}

#[test]
fn map_size_hint_is_bounded_by_remaining_input() {
    assert_eq!(
        from_bytes::<MapHint>(&[0xff, 0xff, 0xff, 0xff, 0x07]),
        Ok(MapHint(Some(0)))
    );

    let observed: ObservedMap = from_bytes(&[2, 1, 10, 2, 20]).unwrap();
    assert_eq!(
        observed,
        ObservedMap {
            hint: Some(2),
            entries: vec![(1, 10), (2, 20)],
        }
    );
}

#[test]
fn huge_claimed_lengths_fail_without_panicking() {
    // MAX_SEQUENCE_LENGTH elements claimed, no payload: must be a clean EOF,
    // for element types of several widths and for byte/string/map forms.
    let header = [0xff, 0xff, 0xff, 0xff, 0x07];
    assert_eq!(from_bytes::<Vec<u8>>(&header), Err(Error::Eof));
    assert_eq!(from_bytes::<Vec<u64>>(&header), Err(Error::Eof));
    assert_eq!(from_bytes::<Vec<String>>(&header), Err(Error::Eof));
    assert_eq!(from_bytes::<String>(&header), Err(Error::Eof));
    assert_eq!(from_bytes::<BTreeMap<u8, u8>>(&header), Err(Error::Eof));
    assert_eq!(from_bytes::<&[u8]>(&header), Err(Error::Eof));

    // One past MAX_SEQUENCE_LENGTH (2^31) is rejected by the length check
    // itself, before any element is read: ULEB128 of 2^31.
    let too_long = [0x80, 0x80, 0x80, 0x80, 0x08];
    let expected = Error::ExceededMaxLen(MAX_SEQUENCE_LENGTH + 1);
    assert_eq!(from_bytes::<Vec<u8>>(&too_long).unwrap_err(), expected);
    assert_eq!(from_bytes::<String>(&too_long).unwrap_err(), expected);
    assert_eq!(
        from_bytes::<BTreeMap<u8, u8>>(&too_long).unwrap_err(),
        expected
    );
    assert_eq!(from_bytes::<&[u8]>(&too_long).unwrap_err(), expected);
}

// ---------------------------------------------------------------------------
// ULEB128 canonical form
// ---------------------------------------------------------------------------

#[test]
fn uleb128_length_boundaries_roundtrip() {
    // (element count of Vec<()>, expected ULEB128 prefix)
    let cases: &[(usize, &[u8])] = &[
        (0, &[0x00]),
        (1, &[0x01]),
        (127, &[0x7f]),
        (128, &[0x80, 0x01]),
        (16_383, &[0xff, 0x7f]),
        (16_384, &[0x80, 0x80, 0x01]),
        (2_097_151, &[0xff, 0xff, 0x7f]),
        (2_097_152, &[0x80, 0x80, 0x80, 0x01]),
    ];
    for (len, prefix) in cases {
        let bytes = to_bytes(&vec![(); *len]).unwrap();
        assert_eq!(&bytes, prefix, "encoding of length {len}");
        assert_eq!(from_bytes::<Vec<()>>(&bytes).unwrap().len(), *len);
    }
}

#[test]
fn uleb128_noncanonical_and_overflowing_lengths_are_rejected() {
    // Non-minimal encodings (trailing zero digit).
    for bytes in [
        &[0x80, 0x00][..],
        &[0x81, 0x00][..],
        &[0x80, 0x80, 0x00][..],
    ] {
        assert_eq!(
            from_bytes::<Vec<u8>>(bytes),
            Err(Error::NonCanonicalUleb128Encoding),
            "input {bytes:02x?}"
        );
    }
    // Longer than a u32 ULEB128 can ever be, and values overflowing u32.
    for bytes in [
        &[0x80, 0x80, 0x80, 0x80, 0x80, 0x01][..],
        &[0xff, 0xff, 0xff, 0xff, 0x1f][..],
        &[0x80, 0x80, 0x80, 0x80, 0x10][..],
    ] {
        assert_eq!(
            from_bytes::<Vec<u8>>(bytes),
            Err(Error::IntegerOverflowDuringUleb128Decoding),
            "input {bytes:02x?}"
        );
    }
    // u32::MAX decodes fine as an integer but exceeds MAX_SEQUENCE_LENGTH.
    assert_eq!(
        from_bytes::<Vec<u8>>(&[0xff, 0xff, 0xff, 0xff, 0x0f]),
        Err(Error::ExceededMaxLen(u32::MAX as usize))
    );
    // Truncated ULEB128 (continuation bit set, then EOF).
    assert_eq!(from_bytes::<Vec<u8>>(&[0x80]), Err(Error::Eof));
    assert_eq!(from_bytes::<Vec<u8>>(&[0x80, 0x80]), Err(Error::Eof));
}

// ---------------------------------------------------------------------------
// Truncation / mutation robustness (panic-freedom)
// ---------------------------------------------------------------------------

#[test]
fn every_truncation_of_a_valid_encoding_is_an_error() {
    let bytes = to_bytes(&sample_record()).unwrap();
    assert_eq!(from_bytes::<Record>(&bytes).unwrap(), sample_record());
    for end in 0..bytes.len() {
        assert!(
            from_bytes::<Record>(&bytes[..end]).is_err(),
            "prefix of {} bytes unexpectedly parsed",
            end
        );
    }
    // Empty input is an EOF error, not a panic, for assorted types.
    assert_eq!(from_bytes::<u8>(&[]), Err(Error::Eof));
    assert_eq!(from_bytes::<bool>(&[]), Err(Error::Eof));
    assert_eq!(from_bytes::<String>(&[]), Err(Error::Eof));
    assert_eq!(from_bytes::<u128>(&[]), Err(Error::Eof));
}

#[test]
fn single_byte_mutations_never_panic_and_stay_canonical() {
    let bytes = to_bytes(&sample_record()).unwrap();
    for pos in 0..bytes.len() {
        for replacement in [0x00, 0xff, bytes[pos] ^ 0xff] {
            let mut mutated = bytes.clone();
            mutated[pos] = replacement;
            // Any mutation that still parses must re-serialize to exactly
            // the mutated bytes: BCS has one encoding per value.
            if let Ok(value) = from_bytes::<Record>(&mutated) {
                assert_eq!(to_bytes(&value).unwrap(), mutated);
            }
        }
    }
}

proptest! {
    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..128)) {
        let _ = from_bytes::<Record>(&bytes);
        let _ = from_bytes::<Vec<Vec<u16>>>(&bytes);
        let _ = from_bytes::<BTreeMap<String, Vec<u32>>>(&bytes);
        let _ = from_bytes::<Option<Record>>(&bytes);
        let _ = from_bytes::<String>(&bytes);
        let _ = from_bytes::<Vec<u8>>(&bytes);
    }

    #[test]
    fn parsed_maps_reencode_to_the_same_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..128)) {
        if let Ok(map) = from_bytes::<BTreeMap<String, u64>>(&bytes) {
            prop_assert_eq!(to_bytes(&map).unwrap(), bytes);
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical maps: byte order, duplicates
// ---------------------------------------------------------------------------

#[test]
fn map_keys_are_ordered_by_serialized_bytes_not_value_order() {
    // As values, [1, 0] < [2]. As BCS bytes, [2] encodes as `01 02` and
    // [1, 0] as `02 01 00`, so [2] must come first on the wire.
    let mut map = BTreeMap::new();
    map.insert(vec![2u8], 10u8);
    map.insert(vec![1u8, 0u8], 20u8);
    let expected = vec![2, 1, 2, 10, 2, 1, 0, 20];
    assert_eq!(to_bytes(&map).unwrap(), expected);
    assert_eq!(from_bytes::<BTreeMap<Vec<u8>, u8>>(&expected), Ok(map));

    // The value-order arrangement is non-canonical and must be rejected,
    // as must a duplicate (serialized) key.
    let value_order = vec![2, 2, 1, 0, 20, 1, 2, 10];
    assert_eq!(
        from_bytes::<BTreeMap<Vec<u8>, u8>>(&value_order),
        Err(Error::NonCanonicalMap)
    );
    let duplicate = vec![2, 1, 2, 10, 1, 2, 20];
    assert_eq!(
        from_bytes::<BTreeMap<Vec<u8>, u8>>(&duplicate),
        Err(Error::NonCanonicalMap)
    );
}

/// Serializes entries in the exact order given, including duplicates.
struct ManualMap(Vec<(u8, u8)>);

impl Serialize for ManualMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

#[test]
fn serializer_rejects_duplicate_map_keys_instead_of_dropping_values() {
    // Regression test: this used to silently drop `(1, 20)` and emit a
    // two-entry map, losing data with no error.
    let dup = ManualMap(vec![(1, 10), (1, 20), (2, 30)]);
    assert_eq!(to_bytes(&dup), Err(Error::NonCanonicalMap));
}

#[test]
fn serializer_sorts_out_of_order_map_keys() {
    let map = ManualMap(vec![(2, 30), (1, 10)]);
    assert_eq!(to_bytes(&map).unwrap(), vec![2, 1, 10, 2, 30]);
}

#[test]
fn hashmap_serialization_is_deterministic() {
    let mut a = HashMap::new();
    let mut b = HashMap::new();
    for i in 0..64u8 {
        a.insert(i, i.wrapping_mul(3));
    }
    for i in (0..64u8).rev() {
        b.insert(i, i.wrapping_mul(3));
    }
    assert_eq!(to_bytes(&a).unwrap(), to_bytes(&b).unwrap());
}

// ---------------------------------------------------------------------------
// Serializer protocol misuse must error, not panic or corrupt output
// ---------------------------------------------------------------------------

struct SeqWithoutLen;
impl Serialize for SeqWithoutLen {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let seq = serializer.serialize_seq(None)?;
        seq.end()
    }
}

struct ValueWithoutKey;
impl Serialize for ValueWithoutKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_value(&1u8)?;
        map.end()
    }
}

struct KeyWithoutValue;
impl Serialize for KeyWithoutValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_key(&1u8)?;
        map.end()
    }
}

struct TwoKeysInARow;
impl Serialize for TwoKeysInARow {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_key(&1u8)?;
        map.serialize_key(&2u8)?;
        map.end()
    }
}

#[test]
fn serializer_protocol_misuse_is_an_error() {
    assert_eq!(to_bytes(&SeqWithoutLen), Err(Error::MissingLen));
    assert_eq!(to_bytes(&ValueWithoutKey), Err(Error::ExpectedMapKey));
    assert_eq!(to_bytes(&KeyWithoutValue), Err(Error::ExpectedMapValue));
    assert_eq!(to_bytes(&TwoKeysInARow), Err(Error::ExpectedMapValue));
}

// ---------------------------------------------------------------------------
// Unsupported types and entry points, in both directions
// ---------------------------------------------------------------------------

struct AnyVisitor;
impl Visitor<'_> for AnyVisitor {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "anything")
    }
}

#[derive(Debug)]
struct CallsDeserializeAny;
impl<'de> Deserialize<'de> for CallsDeserializeAny {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(AnyVisitor)?;
        Ok(Self)
    }
}

#[derive(Debug)]
struct CallsDeserializeIdentifier;
impl<'de> Deserialize<'de> for CallsDeserializeIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(AnyVisitor)?;
        Ok(Self)
    }
}

#[test]
fn unsupported_deserialization_modes_are_rejected() {
    assert_eq!(
        from_bytes::<CallsDeserializeAny>(&[0]).unwrap_err(),
        Error::NotSupported("deserialize_any")
    );
    assert_eq!(
        from_bytes::<serde::de::IgnoredAny>(&[0]),
        Err(Error::NotSupported("deserialize_ignored_any"))
    );
    assert_eq!(
        from_bytes::<CallsDeserializeIdentifier>(&[1, b'a']).unwrap_err(),
        Error::NotSupported("deserialize_identifier")
    );
}

#[test]
fn floats_and_chars_are_rejected_in_both_directions() {
    assert_eq!(to_bytes(&1.0f32), Err(Error::NotSupported("serialize_f32")));
    assert_eq!(to_bytes(&1.0f64), Err(Error::NotSupported("serialize_f64")));
    assert_eq!(to_bytes(&'a'), Err(Error::NotSupported("serialize_char")));
    assert_eq!(
        from_bytes::<f32>(&[0, 0, 0, 0]),
        Err(Error::NotSupported("deserialize_f32"))
    );
    assert_eq!(
        from_bytes::<f64>(&[0; 8]),
        Err(Error::NotSupported("deserialize_f64"))
    );
    assert_eq!(
        from_bytes::<char>(b"a"),
        Err(Error::NotSupported("deserialize_char"))
    );
}

#[test]
fn invalid_scalar_encodings_are_rejected_for_every_bad_byte() {
    for byte in 0..=u8::MAX {
        assert_eq!(
            from_bytes::<bool>(&[byte]).is_ok(),
            byte <= 1,
            "bool byte {byte}"
        );
        let input: &[u8] = if byte == 0 { &[0] } else { &[byte, 42] };
        assert_eq!(
            from_bytes::<Option<u8>>(input).is_ok(),
            byte <= 1,
            "option byte {byte}"
        );
    }
    // Length prefix claims more string bytes than remain.
    assert_eq!(from_bytes::<String>(&[5, b'a']), Err(Error::Eof));
    // Invalid UTF-8 in a string and in borrowed bytes-as-str positions.
    assert_eq!(from_bytes::<String>(&[1, 0xff]), Err(Error::Utf8));
    assert_eq!(from_bytes::<&str>(&[2, 0xc3, 0x28]), Err(Error::Utf8));
}

// ---------------------------------------------------------------------------
// Container-depth limits
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum Tree {
    Leaf,
    Node(Box<Tree>),
}

fn tree(depth: usize) -> Tree {
    let mut t = Tree::Leaf;
    for _ in 0..depth {
        t = Tree::Node(Box::new(t));
    }
    t
}

#[test]
fn enum_depth_limit_applies_symmetrically_to_ser_and_de() {
    // tree(3) is Node(Node(Node(Leaf))): four enum containers deep.
    let value = tree(3);
    assert_eq!(
        to_bytes_with_limit(&value, 3),
        Err(Error::ExceededContainerDepthLimit("Tree"))
    );
    let bytes = to_bytes_with_limit(&value, 4).unwrap();
    assert_eq!(
        from_bytes_with_limit::<Tree>(&bytes, 3),
        Err(Error::ExceededContainerDepthLimit("Tree"))
    );
    assert_eq!(from_bytes_with_limit::<Tree>(&bytes, 4), Ok(value));
}

#[test]
fn sequences_options_and_tuples_do_not_consume_container_depth() {
    // Only structs and enums count towards the depth limit, so these must
    // succeed even with a limit of zero, in both directions.
    let value: Vec<Vec<Vec<u8>>> = vec![vec![vec![1, 2]], vec![]];
    assert_eq!(
        to_bytes_with_limit(&value, 0).unwrap(),
        to_bytes(&value).unwrap()
    );
    assert_eq!(
        from_bytes_with_limit::<Vec<Vec<Vec<u8>>>>(&to_bytes(&value).unwrap(), 0),
        Ok(value)
    );

    let opt: Option<Option<u8>> = Some(Some(7));
    assert_eq!(to_bytes(&opt).unwrap(), vec![1, 1, 7]);
    assert_eq!(
        from_bytes_with_limit::<Option<Option<u8>>>(&[1, 1, 7], 0),
        Ok(opt)
    );
    assert_eq!(to_bytes(&Some(None::<u8>)).unwrap(), vec![1, 0]);

    let tuple: (u8, (String, bool)) = (1, ("x".to_string(), true));
    assert_eq!(
        from_bytes_with_limit::<(u8, (String, bool))>(&to_bytes(&tuple).unwrap(), 0),
        Ok(tuple)
    );

    // A bare primitive also needs no depth budget.
    assert_eq!(from_bytes_with_limit::<u8>(&[7], 0), Ok(7));
}

#[test]
fn depth_limit_above_max_is_rejected_by_every_limited_entry_point() {
    let expected = Error::NotSupported("limit exceeds the max allowed depth 500");
    let too_big = MAX_CONTAINER_DEPTH + 1;
    assert_eq!(to_bytes_with_limit(&1u8, too_big).unwrap_err(), expected);
    assert_eq!(
        from_bytes_with_limit::<u8>(&[1], too_big).unwrap_err(),
        expected
    );
    assert_eq!(
        serialized_size_with_limit(&1u8, too_big).unwrap_err(),
        expected
    );
    assert_eq!(
        from_bytes_seed_with_limit(PhantomData::<u8>, &[1], too_big).unwrap_err(),
        expected
    );
    let mut out = Vec::new();
    assert_eq!(
        serialize_into_with_limit(&mut out, &1u8, too_big).unwrap_err(),
        expected
    );
    // Exactly MAX_CONTAINER_DEPTH remains accepted.
    assert_eq!(to_bytes_with_limit(&1u8, MAX_CONTAINER_DEPTH), Ok(vec![1]));
}

// ---------------------------------------------------------------------------
// Remaining public API surface
// ---------------------------------------------------------------------------

struct FailingWriter;
impl std::io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("boom"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn serialize_into_and_serialized_size_agree_with_to_bytes() {
    for value in [sample_record()] {
        let mut out = Vec::new();
        serialize_into(&mut out, &value).unwrap();
        assert_eq!(out, to_bytes(&value).unwrap());
        assert_eq!(serialized_size(&value).unwrap(), out.len());
        assert_eq!(
            serialized_size_with_limit(&value, MAX_CONTAINER_DEPTH).unwrap(),
            out.len()
        );
        let mut limited = Vec::new();
        serialize_into_with_limit(&mut limited, &value, MAX_CONTAINER_DEPTH).unwrap();
        assert_eq!(limited, out);
    }
    // I/O failures from the writer surface as Error::Io, not a panic.
    match serialize_into(&mut FailingWriter, &1u8) {
        Err(Error::Io(msg)) => assert!(msg.contains("boom"), "msg: {}", msg),
        other => panic!("expected Io error, got {:?}", other),
    }
    assert!(!is_human_readable());
}

#[test]
fn seed_based_deserialization_checks_trailing_bytes_and_limits() {
    let bytes = to_bytes(&vec![1u32, 2, 3]).unwrap();
    assert_eq!(
        from_bytes_seed(PhantomData::<Vec<u32>>, &bytes),
        Ok(vec![1, 2, 3])
    );
    assert_eq!(
        from_bytes_seed_with_limit(PhantomData::<Vec<u32>>, &bytes, 10),
        Ok(vec![1, 2, 3])
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        from_bytes_seed(PhantomData::<Vec<u32>>, &trailing),
        Err(Error::RemainingInput)
    );
}

#[test]
fn empty_and_boundary_values_roundtrip() {
    roundtrip(String::new());
    roundtrip(Vec::<u8>::new());
    roundtrip(BTreeMap::<u8, u8>::new());
    roundtrip(());
    assert_eq!(to_bytes(&()).unwrap(), Vec::<u8>::new());
    assert_eq!(from_bytes::<()>(&[0]), Err(Error::RemainingInput));

    // Integer extremes.
    roundtrip(u8::MAX);
    roundtrip(i8::MIN);
    roundtrip(i16::MIN);
    roundtrip(u16::MAX);
    roundtrip(i32::MIN);
    roundtrip(u32::MAX);
    roundtrip(i64::MIN);
    roundtrip(u64::MAX);
    roundtrip(i128::MIN);
    roundtrip(u128::MAX);
    assert_eq!(to_bytes(&i128::MIN).unwrap(), i128::MIN.to_le_bytes());
    assert_eq!(to_bytes(&u128::MAX).unwrap(), [0xff; 16]);

    // Strings / vectors long enough to need multi-byte ULEB128 lengths,
    // with their contents verified, not just their lengths.
    let long = "a".repeat(16_384);
    let bytes = to_bytes(&long).unwrap();
    assert_eq!(&bytes[..3], &[0x80, 0x80, 0x01]);
    assert_eq!(from_bytes::<String>(&bytes).unwrap(), long);

    let vec: Vec<u16> = (0..300).collect();
    roundtrip(vec);

    // The crate's own test helper exercises the same property.
    bcs::test_helpers::assert_canonical_encode_decode(sample_record());
}
