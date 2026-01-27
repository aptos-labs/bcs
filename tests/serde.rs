// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// For some reason deriving `Arbitrary` results in clippy firing a `unit_arg` violation
#![allow(clippy::unit_arg)]
#![allow(non_local_definitions)]
// Allow these clippy warnings for test code
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::zero_sized_map_values)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::owned_cow)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use proptest::prelude::*;
use proptest_derive::Arbitrary;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use bcs::{
    from_bytes, from_bytes_with_limit, serialized_size, to_bytes, to_bytes_with_limit, Error,
    MAX_CONTAINER_DEPTH, MAX_SEQUENCE_LENGTH,
};

fn is_same<T>(t: T)
where
    T: Serialize + DeserializeOwned + fmt::Debug + PartialEq,
{
    let bytes = to_bytes(&t).unwrap();
    let s: T = from_bytes(&bytes).unwrap();
    assert_eq!(t, s);
    assert_eq!(bytes.len(), serialized_size(&t).unwrap());
}

// TODO deriving `Arbitrary` is currently broken for enum types
// Once AltSysrq/proptest#163 is merged we can use `Arbitrary` again.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum E {
    Unit,
    Newtype(u16),
    Tuple(u16, u16),
    Struct { a: u32 },
}

#[test]
fn test_enum() {
    let u = E::Unit;
    let expected = vec![0];
    assert_eq!(to_bytes(&u).unwrap(), expected);
    is_same(u);

    let n = E::Newtype(1);
    let expected = vec![1, 1, 0];
    assert_eq!(to_bytes(&n).unwrap(), expected);
    is_same(n);

    let t = E::Tuple(1, 2);
    let expected = vec![2, 1, 0, 2, 0];
    assert_eq!(to_bytes(&t).unwrap(), expected);
    is_same(t);

    let s = E::Struct { a: 1 };
    let expected = vec![3, 1, 0, 0, 0];
    assert_eq!(to_bytes(&s).unwrap(), expected);
    is_same(s);
}

#[derive(Arbitrary, Debug, Deserialize, Serialize, PartialEq)]
struct S {
    int: u16,
    option: Option<u8>,
    seq: Vec<String>,
    boolean: bool,
}

proptest! {
    #[test]
    fn proptest_bool(v in any::<bool>()) {
        assert_eq!(to_bytes(&v)?, vec![u8::from(v)]);
        is_same(v);
    }

    #[test]
    fn proptest_i8(v in any::<i8>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_i16(v in any::<i16>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_i32(v in any::<i32>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_i64(v in any::<i64>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_i128(v in any::<i128>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_u8(v in any::<u8>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_u16(v in any::<u16>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_u32(v in any::<u32>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_u64(v in any::<u64>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_u128(v in any::<u128>()) {
        assert_eq!(to_bytes(&v)?, v.to_le_bytes());
        is_same(v);
    }

    #[test]
    fn proptest_string(v in any::<String>()) {
        let mut expected = Vec::with_capacity(v.len() + 4);
        // Larger lengths have more complex uleb128 encodings.
        prop_assume!(v.len() < 128);
        expected.extend_from_slice(&(v.len() as u8).to_le_bytes());
        expected.extend_from_slice(v.as_bytes());
        assert_eq!(to_bytes(&v)?, expected);

        is_same(v);
    }

    #[test]
    fn proptest_vec(v in any::<Vec<u8>>()) {
        let mut expected = Vec::with_capacity(v.len() + 4);
        // Larger lengths have more complex uleb128 encodings.
        prop_assume!(v.len() < 128);
        expected.extend_from_slice(&(v.len() as u8).to_le_bytes());
        expected.extend_from_slice(&v);
        assert_eq!(to_bytes(&v)?, expected);

        is_same(v);
    }

    #[test]
    fn proptest_option(v in any::<Option<u8>>()) {
        let expected = v.map_or_else(|| vec![0], |v| vec![1, v]);
        assert_eq!(to_bytes(&v)?, expected);

        is_same(v);
    }

    #[test]
    fn proptest_btreemap(v in any::<BTreeMap<Vec<u8>, Vec<u8>>>()) {
        is_same(v);
    }

    #[test]
    fn proptest_tuple2(v in any::<(i16, String)>()) {
        is_same(v);
    }

    #[test]
    fn proptest_tuple3(v in any::<(bool, u32, String)>()) {
        is_same(v);
    }

    #[test]
    fn proptest_tuple4(v in any::<(bool, u32, Option<i64>)>()) {
        is_same(v);
    }

    #[test]
    fn proptest_tuple_strings(v in any::<(String, String, String)>()) {
        is_same(v);
    }

    #[test]
    fn proptest_lexicographic_order(v in any::<BTreeMap<Vec<u8>, Vec<u8>>>()) {
        let bytes = to_bytes(&v).unwrap();
        // This test assumes small maps and small vectors.
        // This is what proptest always generates in practice but we will make
        // the assumptions explicit anyway.
        prop_assume!(v.len() < 128);

        let m : BTreeMap<Vec<u8>, Vec<u8>> = v.iter().filter_map(|(k, v)| {
            if k.len() >= 128 || v.len() >= 128 {
                return None;
            }
            let mut k_bytes = Vec::with_capacity(k.len() + 4);
            k_bytes.extend_from_slice(&(k.len() as u8).to_le_bytes());
            k_bytes.extend(k.iter());
            let mut v_bytes = Vec::with_capacity(v.len() + 4);
            v_bytes.extend_from_slice(&(v.len() as u8).to_le_bytes());
            v_bytes.extend(v.iter());

            Some((k_bytes, v_bytes))
        })
        .collect();
        prop_assume!(v.len() == m.len());

        let mut expected = Vec::with_capacity(bytes.len());
        expected.extend_from_slice(&(m.len() as u8).to_le_bytes());
        for (key, value) in m {
            expected.extend(key.iter());
            expected.extend(value.iter());
        }

        assert_eq!(expected, bytes);
    }

    #[test]
    fn proptest_box(v in any::<Box<u32>>()) {
        is_same(v);
    }

    #[test]
    fn proptest_struct(v in any::<S>()) {
        is_same(v);
    }

    #[test]
    fn proptest_addr(v in any::<Addr>()) {
        is_same(v);
    }

    #[test]
    fn proptest_bar(v in any::<Bar>()) {
        is_same(v);
    }

    #[test]
    fn proptest_foo(v in any::<Foo>()) {
        is_same(v);
    }
}

#[test]
fn invalid_utf8() {
    let invalid_utf8 = vec![1, 0xFF];
    assert_eq!(from_bytes::<String>(&invalid_utf8), Err(Error::Utf8));
}

#[test]
fn uleb_encoding_and_variant() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Test {
        One,
        Two,
    }

    let valid_variant = vec![1];
    from_bytes::<Test>(&valid_variant).unwrap();

    let invalid_variant = vec![5];
    // Error comes from serde
    assert_eq!(
        from_bytes::<Test>(&invalid_variant),
        Err(Error::Custom(
            "invalid value: integer `5`, expected variant index 0 <= i < 2".into()
        ))
    );

    let invalid_bytes = vec![0x80, 0x80, 0x80, 0x80];
    // Error is due to EOF.
    assert_eq!(from_bytes::<Test>(&invalid_bytes), Err(Error::Eof));

    let invalid_uleb = vec![0x80, 0x80, 0x80, 0x80, 0x80];
    // Error comes from uleb decoder because u32 are never that long.
    assert_eq!(
        from_bytes::<Test>(&invalid_uleb),
        Err(Error::IntegerOverflowDuringUleb128Decoding)
    );

    let invalid_uleb = vec![0x80, 0x80, 0x80, 0x80, 0x1f];
    // Error comes from uleb decoder because we are truncating a larger integer into u32.
    assert_eq!(
        from_bytes::<Test>(&invalid_uleb),
        Err(Error::IntegerOverflowDuringUleb128Decoding)
    );

    let invalid_uleb = vec![0x80, 0x80, 0x80, 0x80, 0x0f];
    // Error comes from Serde because ULEB integer is valid.
    assert_eq!(
        from_bytes::<Test>(&invalid_uleb),
        Err(Error::Custom(
            "invalid value: integer `4026531840`, expected variant index 0 <= i < 2".into()
        ))
    );

    let invalid_uleb = vec![0x80, 0x80, 0x80, 0x00];
    // Uleb decoder must reject non-canonical forms.
    assert_eq!(
        from_bytes::<Test>(&invalid_uleb),
        Err(Error::NonCanonicalUleb128Encoding)
    );
}

#[test]
fn invalid_option() {
    let invalid_option = vec![5, 0];
    assert_eq!(
        from_bytes::<Option<u8>>(&invalid_option),
        Err(Error::ExpectedOption)
    );
}

#[test]
fn invalid_bool() {
    let invalid_bool = vec![9];
    assert_eq!(
        from_bytes::<bool>(&invalid_bool),
        Err(Error::ExpectedBoolean)
    );
}

#[test]
fn sequence_too_long() {
    let seq = vec![0; MAX_SEQUENCE_LENGTH + 1];
    match to_bytes(&seq).unwrap_err() {
        Error::ExceededMaxLen(len) => assert_eq!(len, MAX_SEQUENCE_LENGTH + 1),
        _ => panic!(),
    }
}

#[test]
fn variable_lengths() {
    assert_eq!(to_bytes(&vec![(); 1]).unwrap(), vec![0x01]);
    assert_eq!(to_bytes(&vec![(); 128]).unwrap(), vec![0x80, 0x01]);
    assert_eq!(to_bytes(&vec![(); 255]).unwrap(), vec![0xff, 0x01]);
    assert_eq!(
        to_bytes(&vec![(); 786_432]).unwrap(),
        vec![0x80, 0x80, 0x30]
    );
}

#[test]
fn sequence_not_long_enough() {
    let seq = vec![5, 1, 2, 3, 4]; // Missing 5th element
    assert_eq!(from_bytes::<Vec<u8>>(&seq), Err(Error::Eof));
}

#[test]
fn map_not_canonical() {
    let mut map = BTreeMap::new();
    map.insert(4u8, ());
    map.insert(5u8, ());
    let seq = vec![2, 4, 5];
    assert_eq!(from_bytes::<BTreeMap<u8, ()>>(&seq), Ok(map));
    // Make sure out-of-order keys are rejected.
    let seq = vec![2, 5, 4];
    assert_eq!(
        from_bytes::<BTreeMap<u8, ()>>(&seq),
        Err(Error::NonCanonicalMap)
    );
    // Make sure duplicate keys are rejected.
    let seq = vec![2, 5, 5];
    assert_eq!(
        from_bytes::<BTreeMap<u8, ()>>(&seq),
        Err(Error::NonCanonicalMap)
    );
}

#[test]
fn by_default_btreesets_are_serialized_as_sequences() {
    // See https://docs.serde.rs/src/serde/de/impls.rs.html
    // This is a big caveat for us, but luckily, generate-format will track this in the YAML output.
    let mut set = BTreeSet::new();
    set.insert(4u8);
    set.insert(5u8);
    let seq = vec![2, 4, 5];
    assert_eq!(from_bytes::<BTreeSet<u8>>(&seq), Ok(set.clone()));
    let seq = vec![2, 5, 4];
    assert_eq!(from_bytes::<BTreeSet<u8>>(&seq), Ok(set.clone()));
    // Duplicate keys are just ok.
    let seq = vec![3, 5, 5, 4];
    assert_eq!(from_bytes::<BTreeSet<u8>>(&seq), Ok(set));
}

#[test]
fn leftover_bytes() {
    let seq = vec![5, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]; // 5 extra elements
    assert_eq!(from_bytes::<Vec<u8>>(&seq), Err(Error::RemainingInput));
}

#[test]
fn test_f32() {
    assert!(to_bytes(&1.0f32).is_err());
}

#[test]
fn test_f64() {
    assert!(to_bytes(&42.0f64).is_err());
}

#[test]
fn test_char() {
    assert!(to_bytes(&'a').is_err());
}

#[test]
fn zero_copy_parse() {
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo<'a> {
        borrowed_str: &'a str,
        borrowed_bytes: &'a [u8],
    }

    let f = Foo {
        borrowed_str: "hi",
        borrowed_bytes: &[0, 1, 2, 3],
    };
    {
        let expected = vec![2, b'h', b'i', 4, 0, 1, 2, 3];
        let encoded = to_bytes(&f).unwrap();
        assert_eq!(expected, encoded);
        let out: Foo = from_bytes(&encoded[..]).unwrap();
        assert_eq!(out, f);
    }
}

#[test]
fn cow() {
    use std::borrow::Cow;

    let large_object = vec![1u32, 2, 3, 4, 5, 6];
    let mut large_map = BTreeMap::new();
    large_map.insert(1, 2);

    #[derive(Serialize, Deserialize, Debug)]
    enum Message<'a> {
        M1(Cow<'a, Vec<u32>>),
        M2(Cow<'a, BTreeMap<u32, u32>>),
    }

    // M1
    {
        let serialized = to_bytes(&Message::M1(Cow::Borrowed(&large_object))).unwrap();
        let deserialized: Message<'static> = from_bytes(&serialized).unwrap();

        match deserialized {
            Message::M1(b) => assert_eq!(b.into_owned(), large_object),
            _ => panic!(),
        }
    }

    // M2
    {
        let serialized = to_bytes(&Message::M2(Cow::Borrowed(&large_map))).unwrap();
        let deserialized: Message<'static> = from_bytes(&serialized).unwrap();

        match deserialized {
            Message::M2(b) => assert_eq!(b.into_owned(), large_map),
            _ => panic!(),
        }
    }
}

#[test]
fn strbox() {
    use std::borrow::Cow;

    let strx: &'static str = "hello world";
    let serialized = to_bytes(&Cow::Borrowed(strx)).unwrap();
    let deserialized: Cow<'static, String> = from_bytes(&serialized).unwrap();
    let stringx: String = deserialized.into_owned();
    assert_eq!(strx, stringx);
}

#[test]
fn slicebox() {
    use std::borrow::Cow;

    let slice = [1u32, 2, 3, 4, 5];
    let serialized = to_bytes(&Cow::Borrowed(&slice[..])).unwrap();
    let deserialized: Cow<'static, Vec<u32>> = from_bytes(&serialized).unwrap();
    {
        let sb: &[u32] = &deserialized;
        assert_eq!(slice, sb);
    }
    let vecx: Vec<u32> = deserialized.into_owned();
    assert_eq!(slice, vecx[..]);
}

#[test]
fn path_buf() {
    use std::path::{Path, PathBuf};

    let path = Path::new("foo").to_path_buf();
    let encoded = to_bytes(&path).unwrap();
    let decoded: PathBuf = from_bytes(&encoded).unwrap();
    assert!(path.to_str() == decoded.to_str());
}

#[derive(Arbitrary, Debug, Deserialize, Serialize, PartialEq)]
struct Addr([u8; 32]);

#[derive(Arbitrary, Debug, Deserialize, Serialize, PartialEq)]
struct Bar {
    a: u64,
    b: Vec<u8>,
    c: Addr,
    d: u32,
}

#[derive(Arbitrary, Debug, Deserialize, Serialize, PartialEq)]
struct Foo {
    a: u64,
    b: Vec<u8>,
    c: Bar,
    d: bool,
    e: BTreeMap<Vec<u8>, Vec<u8>>,
}

#[test]
fn serde_known_vector() {
    let b = Bar {
        a: 100,
        b: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
        c: Addr([5u8; 32]),
        d: 99,
    };

    let mut map = BTreeMap::new();
    map.insert(vec![0, 56, 21], vec![22, 10, 5]);
    map.insert(vec![1], vec![22, 21, 67]);
    map.insert(vec![20, 21, 89, 105], vec![201, 23, 90]);

    let f = Foo {
        a: u64::MAX,
        b: vec![100, 99, 88, 77, 66, 55],
        c: b,
        d: true,
        e: map,
    };

    let bytes = to_bytes(&f).unwrap();

    let test_vector = vec![
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x06, 0x64, 0x63, 0x58, 0x4d, 0x42, 0x37,
        0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
        0x06, 0x07, 0x08, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
        0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
        0x05, 0x05, 0x05, 0x05, 0x05, 0x63, 0x00, 0x00, 0x00, 0x01, 0x03, 0x01, 0x01, 0x03, 0x16,
        0x15, 0x43, 0x03, 0x00, 0x38, 0x15, 0x03, 0x16, 0x0a, 0x05, 0x04, 0x14, 0x15, 0x59, 0x69,
        0x03, 0xc9, 0x17, 0x5a,
    ];

    // make sure we serialize into exact same bytes as before
    assert_eq!(test_vector, bytes);

    // make sure we can deserialize the test vector into expected struct
    let deserialized_foo: Foo = from_bytes(&test_vector).unwrap();
    assert_eq!(f, deserialized_foo);
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
struct List {
    next: Option<(usize, Box<List>)>,
}

impl List {
    fn empty() -> Self {
        Self { next: None }
    }

    fn cons(value: usize, tail: List) -> Self {
        Self {
            next: Some((value, Box::new(tail))),
        }
    }

    fn integers(len: usize) -> Self {
        if len == 0 {
            Self::empty()
        } else {
            Self::cons(len - 1, Self::integers(len - 1))
        }
    }
}

#[test]
fn test_recursion_limit() {
    let l1 = List::integers(4);
    let b1 = to_bytes(&l1).unwrap();
    assert_eq!(
        b1,
        vec![
            1, 3, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 0
        ]
    );
    assert_eq!(from_bytes::<List>(&b1).unwrap(), l1);

    let l2 = List::integers(MAX_CONTAINER_DEPTH - 1);
    let b2 = to_bytes(&l2).unwrap();
    assert_eq!(from_bytes::<List>(&b2).unwrap(), l2);

    let l3 = List::integers(MAX_CONTAINER_DEPTH);
    assert_eq!(
        to_bytes(&l3),
        Err(Error::ExceededContainerDepthLimit("List"))
    );
    let mut b3 = vec![1, 243, 1, 0, 0, 0, 0, 0, 0];
    b3.extend(b2);
    assert_eq!(
        from_bytes::<List>(&b3),
        Err(Error::ExceededContainerDepthLimit("List"))
    );

    let b2_pair = to_bytes(&(&l2, &l2)).unwrap();
    assert_eq!(
        from_bytes::<(List, List)>(&b2_pair).unwrap(),
        (l2.clone(), l2.clone())
    );
    assert_eq!(
        to_bytes(&(&l2, &l3)),
        Err(Error::ExceededContainerDepthLimit("List"))
    );
    assert_eq!(
        to_bytes(&(&l3, &l2)),
        Err(Error::ExceededContainerDepthLimit("List"))
    );
    assert_eq!(
        to_bytes(&(&l3, &l3)),
        Err(Error::ExceededContainerDepthLimit("List"))
    );

    // test customized limit
    let limit = 100;
    let not_supported_err = Error::NotSupported("limit exceeds the max allowed depth 500");
    let l4 = List::integers(limit);
    assert_eq!(
        to_bytes_with_limit(&l4, limit),
        Err(Error::ExceededContainerDepthLimit("List"))
    );
    assert_eq!(
        to_bytes_with_limit(&l4, MAX_CONTAINER_DEPTH + 1),
        Err(not_supported_err.clone()),
    );
    let bytes = to_bytes_with_limit(&l4, limit + 1).unwrap();
    assert_eq!(
        from_bytes_with_limit::<List>(&bytes, limit),
        Err(Error::ExceededContainerDepthLimit("List"))
    );
    assert_eq!(from_bytes_with_limit(&bytes, limit + 1), Ok(l4));
    assert_eq!(
        from_bytes_with_limit::<List>(&bytes, MAX_CONTAINER_DEPTH + 1),
        Err(not_supported_err)
    );
}

#[derive(Deserialize, Serialize)]
enum EnumA {
    ValueA,
}

#[test]
fn test_recursion_limit_enum() {
    let a = EnumA::ValueA;

    to_bytes_with_limit(&a, 0).unwrap_err();

    let bytes = to_bytes_with_limit(&a, 1).unwrap();
    let _: EnumA = from_bytes_with_limit(&bytes, 1).unwrap();
}

// ============================================================================
// Additional tests for 100% coverage
// ============================================================================

#[test]
fn test_to_bytes_with_capacity() {
    use bcs::to_bytes_with_capacity;

    let data = vec![1u32, 2, 3, 4, 5];
    let bytes = to_bytes_with_capacity(&data, 100).unwrap();
    let expected = to_bytes(&data).unwrap();
    assert_eq!(bytes, expected);
}

#[test]
fn test_serialized_size_with_limit() {
    use bcs::{serialized_size, serialized_size_with_limit};

    let data = vec![1u32, 2, 3];
    let size = serialized_size_with_limit(&data, 10).unwrap();
    assert_eq!(size, serialized_size(&data).unwrap());

    // Test exceeding limit
    let err = serialized_size_with_limit(&data, 501).unwrap_err();
    assert!(matches!(err, Error::NotSupported(_)));
}

#[test]
fn test_serialize_into_with_limit_exceeding() {
    use bcs::serialize_into_with_limit;

    let data = 42u32;
    let mut output = Vec::new();
    let err = serialize_into_with_limit(&mut output, &data, 501).unwrap_err();
    assert!(matches!(err, Error::NotSupported(_)));
}

#[test]
fn test_is_human_readable() {
    assert!(!bcs::is_human_readable());
}

#[test]
fn test_from_bytes_seed() {
    use bcs::from_bytes_seed;
    use serde::de::DeserializeSeed;

    struct U32Seed;

    impl<'de> DeserializeSeed<'de> for U32Seed {
        type Value = u32;

        fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            u32::deserialize(deserializer)
        }
    }

    let bytes = to_bytes(&42u32).unwrap();
    let result: u32 = from_bytes_seed(U32Seed, &bytes).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_from_bytes_seed_with_limit() {
    use bcs::from_bytes_seed_with_limit;
    use serde::de::DeserializeSeed;

    struct U32Seed;

    impl<'de> DeserializeSeed<'de> for U32Seed {
        type Value = u32;

        fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            u32::deserialize(deserializer)
        }
    }

    let bytes = to_bytes(&42u32).unwrap();
    let result: u32 = from_bytes_seed_with_limit(U32Seed, &bytes, 10).unwrap();
    assert_eq!(result, 42);

    // Test exceeding limit
    let err = from_bytes_seed_with_limit(U32Seed, &bytes, 501).unwrap_err();
    assert!(matches!(err, Error::NotSupported(_)));
}

#[test]
fn test_unit_struct_serde() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct UnitStruct;

    let bytes = to_bytes(&UnitStruct).unwrap();
    let result: UnitStruct = from_bytes(&bytes).unwrap();
    assert_eq!(result, UnitStruct);
}

#[test]
fn test_tuple_struct_serde() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TupleStruct(u32, String);

    let data = TupleStruct(42, "hello".to_string());
    let bytes = to_bytes(&data).unwrap();
    let result: TupleStruct = from_bytes(&bytes).unwrap();
    assert_eq!(result, data);
}

#[test]
fn test_eof_error_in_read_bytes() {
    // Try to deserialize with insufficient bytes
    let bytes = vec![0x05]; // Length says 5 bytes, but only 1 byte available
    let err = from_bytes::<Vec<u8>>(&bytes).unwrap_err();
    assert!(matches!(err, Error::Eof));
}

#[test]
fn test_test_helpers() {
    use bcs::test_helpers::assert_canonical_encode_decode;

    assert_canonical_encode_decode(&42u32);
    assert_canonical_encode_decode(&"hello".to_string());
    assert_canonical_encode_decode(&vec![1, 2, 3]);
}

#[test]
fn test_map_serialization_error_paths() {
    // Test empty map
    let map: BTreeMap<u32, u32> = BTreeMap::new();
    let bytes = to_bytes(&map).unwrap();
    let result: BTreeMap<u32, u32> = from_bytes(&bytes).unwrap();
    assert_eq!(result, map);
}

#[test]
fn test_byte_buf_deserialization() {
    // Test deserializing into Vec<u8> (uses deserialize_byte_buf path)
    let data: Vec<u8> = vec![1, 2, 3, 4, 5];
    let bytes = to_bytes(&data).unwrap();
    let result: Vec<u8> = from_bytes(&bytes).unwrap();
    assert_eq!(result, data);
}

#[test]
fn test_custom_error() {
    // Test Error::Custom path via serde
    let err: Error = <Error as serde::de::Error>::custom("test error");
    assert!(matches!(err, Error::Custom(_)));
}

#[test]
fn test_io_error_conversion() {
    use std::io;

    let io_err = io::Error::other("test");
    let bcs_err: Error = io_err.into();
    assert!(matches!(bcs_err, Error::Io(_)));
}

#[test]
fn test_serialization_custom_error() {
    // Test ser::Error::custom path
    let err: Error = <Error as serde::ser::Error>::custom("serialization error");
    assert!(matches!(err, Error::Custom(_)));
}

#[test]
fn test_exceeded_max_sequence_length() {
    // Create bytes that claim a huge sequence length
    // ULEB128 encoding of MAX_SEQUENCE_LENGTH + 1 = 2^31
    let bytes = vec![0x80, 0x80, 0x80, 0x80, 0x08]; // 2^31 in ULEB128
    let err = from_bytes::<Vec<u8>>(&bytes).unwrap_err();
    assert!(matches!(err, Error::ExceededMaxLen(_)));
}

// ============================================================================
// Security tests
// ============================================================================

#[test]
fn test_memory_amplification_protection() {
    // Security test: Ensure that claiming a large length with insufficient data
    // returns Eof quickly without attempting to allocate large amounts of memory.
    //
    // A 5-byte payload claiming 1 million elements should fail immediately,
    // not after trying to allocate memory for 1 million elements.

    // ULEB128 encoding of 1,000,000 = 0xF4240
    // 0x40 | 0x80 = 0xC0, 0x84 | 0x80 = 0x84 (wait, let me recalculate)
    // 1,000,000 = 0xF4240
    // Byte 0: 0x40 | 0x80 = 0xC0 (bits 0-6 = 64)
    // Byte 1: 0x48 | 0x80 = 0xC8 (bits 7-13 = 72) -- wait let me do this properly
    // 1,000,000 in binary: 11110100001001000000
    // Split into 7-bit groups from LSB: 1000000 (64), 0100100 (36), 0111101 (61)
    // So: 0xC0 (64 + 0x80), 0xA4 (36 + 0x80), 0x3D (61, no continuation)
    let bytes = vec![0xC0, 0x84, 0x3D]; // ULEB128 for 1,000,000

    let err = from_bytes::<Vec<u8>>(&bytes).unwrap_err();
    // Should fail with Eof because we only have 0 bytes of actual data,
    // but claimed 1,000,000 bytes
    assert!(matches!(err, Error::Eof));
}

#[test]
fn test_memory_amplification_protection_string() {
    // Test the same protection for strings
    // Claim 10,000 bytes but provide only 2
    let bytes = vec![0x90, 0x4E, b'h', b'i']; // ULEB128 for 10,000, then "hi"

    let err = from_bytes::<String>(&bytes).unwrap_err();
    assert!(matches!(err, Error::Eof));
}

#[test]
fn test_memory_amplification_small_valid_length() {
    // Make sure valid small lengths still work
    let bytes = vec![0x02, 0x41, 0x42]; // length 2, then "AB"
    let result: Vec<u8> = from_bytes(&bytes).unwrap();
    assert_eq!(result, vec![0x41, 0x42]);
}

#[test]
fn test_duplicate_map_keys_serialization() {
    // Create a HashMap and manually serialize it to test the duplicate detection.
    // Since HashMap doesn't allow duplicate keys directly, we need to test via
    // serde's serialize_map interface with a custom type.
    #[derive(Debug)]
    struct DuplicateKeyMap;

    impl Serialize for DuplicateKeyMap {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeMap;
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry(&1u32, &"first")?;
            map.serialize_entry(&1u32, &"second")?; // Duplicate key!
            map.end()
        }
    }

    let err = to_bytes(&DuplicateKeyMap).unwrap_err();
    assert!(matches!(err, Error::NonCanonicalMap));
}

#[test]
fn test_map_serialization_no_duplicates() {
    use std::collections::HashMap;

    // Normal map without duplicates should work fine
    let mut map = HashMap::new();
    map.insert(1u32, "one");
    map.insert(2u32, "two");
    map.insert(3u32, "three");

    let bytes = to_bytes(&map).unwrap();
    let result: HashMap<u32, String> = from_bytes(&bytes).unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(result.get(&1).map(String::as_str), Some("one"));
    assert_eq!(result.get(&2).map(String::as_str), Some("two"));
    assert_eq!(result.get(&3).map(String::as_str), Some("three"));
}
