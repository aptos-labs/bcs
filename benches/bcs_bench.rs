// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use bcs::{from_bytes, to_bytes, to_bytes_with_capacity};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize, Deserialize, Clone)]
struct SimpleStruct {
    a: u64,
    b: u32,
    c: u16,
    d: u8,
    e: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct ComplexStruct {
    id: u64,
    name: String,
    values: Vec<u64>,
    nested: Option<SimpleStruct>,
}

/// Benchmarks for BCS serialization.
///
/// # Panics
///
/// Panics if any serialization operation fails unexpectedly.
pub fn serialize_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");

    // Primitive types
    let u64_val: u64 = 0x1234_5678_90AB_CDEF;
    group.bench_function("u64", |b| b.iter(|| to_bytes(black_box(&u64_val)).unwrap()));

    // Simple struct
    let simple = SimpleStruct {
        a: 12_345_678_901_234,
        b: 1_234_567_890,
        c: 12345,
        d: 123,
        e: true,
    };
    group.bench_function("simple_struct", |b| {
        b.iter(|| to_bytes(black_box(&simple)).unwrap());
    });

    // Complex struct with nested data
    let complex = ComplexStruct {
        id: 42,
        name: "benchmark test string".to_string(),
        values: (0..100).collect(),
        nested: Some(simple.clone()),
    };
    group.bench_function("complex_struct", |b| {
        b.iter(|| to_bytes(black_box(&complex)).unwrap());
    });

    // Test pre-allocation benefit
    let serialized_size = to_bytes(&complex).unwrap().len();
    group.bench_function("complex_struct_with_capacity", |b| {
        b.iter(|| to_bytes_with_capacity(black_box(&complex), serialized_size).unwrap());
    });

    // Vec of u64s at various sizes
    for size in &[10_u64, 100, 1000, 10000] {
        let vec: Vec<u64> = (0..*size).collect();
        group.throughput(Throughput::Elements(*size));
        group.bench_with_input(BenchmarkId::new("vec_u64", size), &vec, |b, v| {
            b.iter(|| to_bytes(black_box(v)).unwrap());
        });
    }

    // String serialization
    let short_string = "hello".to_string();
    let long_string = "a".repeat(1000);
    group.bench_function("short_string", |b| {
        b.iter(|| to_bytes(black_box(&short_string)).unwrap());
    });
    group.bench_function("long_string", |b| {
        b.iter(|| to_bytes(black_box(&long_string)).unwrap());
    });

    // Maps
    let mut btree_map = BTreeMap::new();
    let mut hash_map = HashMap::new();
    for i in 0u32..2000u32 {
        btree_map.insert(i, i);
        hash_map.insert(i, i);
    }
    group.bench_function("btree_map_2000", |b| {
        b.iter(|| to_bytes(black_box(&btree_map)).unwrap());
    });
    group.bench_function("hash_map_2000", |b| {
        b.iter(|| to_bytes(black_box(&hash_map)).unwrap());
    });

    group.finish();
}

/// Benchmarks for BCS deserialization.
///
/// # Panics
///
/// Panics if any serialization or deserialization operation fails unexpectedly.
pub fn deserialize_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    // Primitive types
    let u64_bytes = to_bytes(&0x1234_5678_90AB_CDEF_u64).unwrap();
    group.bench_function("u64", |b| {
        b.iter(|| from_bytes::<u64>(black_box(&u64_bytes)).unwrap());
    });

    // Simple struct
    let simple = SimpleStruct {
        a: 12_345_678_901_234,
        b: 1_234_567_890,
        c: 12345,
        d: 123,
        e: true,
    };
    let simple_bytes = to_bytes(&simple).unwrap();
    group.bench_function("simple_struct", |b| {
        b.iter(|| from_bytes::<SimpleStruct>(black_box(&simple_bytes)).unwrap());
    });

    // Complex struct
    let complex = ComplexStruct {
        id: 42,
        name: "benchmark test string".to_string(),
        values: (0..100).collect(),
        nested: Some(simple.clone()),
    };
    let complex_bytes = to_bytes(&complex).unwrap();
    group.bench_function("complex_struct", |b| {
        b.iter(|| from_bytes::<ComplexStruct>(black_box(&complex_bytes)).unwrap());
    });

    // Vec of u64s at various sizes
    for size in &[10_u64, 100, 1000, 10000] {
        let vec: Vec<u64> = (0..*size).collect();
        let vec_bytes = to_bytes(&vec).unwrap();
        group.throughput(Throughput::Elements(*size));
        group.bench_with_input(BenchmarkId::new("vec_u64", size), &vec_bytes, |b, bytes| {
            b.iter(|| from_bytes::<Vec<u64>>(black_box(bytes)).unwrap());
        });
    }

    // String deserialization
    let short_string_bytes = to_bytes(&"hello".to_string()).unwrap();
    let long_string_bytes = to_bytes(&"a".repeat(1000)).unwrap();
    group.bench_function("short_string", |b| {
        b.iter(|| from_bytes::<String>(black_box(&short_string_bytes)).unwrap());
    });
    group.bench_function("long_string", |b| {
        b.iter(|| from_bytes::<String>(black_box(&long_string_bytes)).unwrap());
    });

    // Maps
    let mut btree_map = BTreeMap::new();
    for i in 0u32..2000u32 {
        btree_map.insert(i, i);
    }
    let map_bytes = to_bytes(&btree_map).unwrap();
    group.bench_function("btree_map_2000", |b| {
        b.iter(|| from_bytes::<BTreeMap<u32, u32>>(black_box(&map_bytes)).unwrap());
    });

    group.finish();
}

criterion_group!(benches, serialize_benchmarks, deserialize_benchmarks);
criterion_main!(benches);
