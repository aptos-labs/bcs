// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

/// Asserts that a value can be serialized and deserialized back to an equal value.
///
/// This is a test helper function that verifies round-trip serialization.
///
/// # Panics
///
/// Panics if:
/// - Serialization fails
/// - Deserialization fails
/// - The deserialized value is not equal to the original
// By value on purpose: this is the established public signature of this
// helper, and the value is compared against its round-trip result below.
#[allow(clippy::needless_pass_by_value)]
pub fn assert_canonical_encode_decode<T>(t: T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    let bytes = crate::to_bytes(&t).unwrap();
    let s: T = crate::from_bytes(&bytes).unwrap();
    assert_eq!(t, s);
}
