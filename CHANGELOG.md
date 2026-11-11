# Changelog

## [Unreleased]

## [v0.1.4] - 2026-11-10

### Security

- **Memory amplification protection**: Deserialization now validates that claimed byte-sequence/string lengths (byte counts) are plausible given the remaining input, preventing DoS attacks where a small malicious payload could trigger large memory allocations. Element counts for sequences/maps are not byte counts (zero-sized elements encode as a length prefix alone) and remain bounded by `MAX_SEQUENCE_LENGTH` plus serde's capped `size_hint` pre-allocation.
- **Duplicate map key detection**: Serialization now returns `Error::NonCanonicalMap` when duplicate keys are encountered instead of silently dropping duplicates, ensuring data integrity.

### Added

- `from_bytes_seed_prefix()` function to decode one value from the front of a buffer and return it with the number of bytes consumed, allowing trailing bytes. This reads values concatenated without length prefixes, where each value's end is known only by decoding it.
- Customizable container-depth limits via `from_bytes_with_limit()`, `from_bytes_seed_with_limit()`, `to_bytes_with_limit()`, `serialize_into_with_limit()`, and `serialized_size_with_limit()`, instead of only the fixed `MAX_CONTAINER_DEPTH`.
- `to_bytes_with_capacity()` function for pre-allocating output buffers when the serialized size is known or estimated, reducing allocations.
- Comprehensive `# Errors` documentation sections on all public functions.
- `#[must_use]` attribute on `is_human_readable()`.
- Explicit `#![forbid(unsafe_code)]` via Cargo.toml lints section.
- Full pedantic clippy lint compliance with minimal, justified exceptions for binary serialization casts.
- `rustfmt.toml` configuration for consistent code formatting.

### Fixed

- Enum container-depth (nesting) checks now match between serialization and deserialization.

### Changed

- **Optimized ULEB128 encoding/decoding**: Added fast paths for single-byte values (0-127), which are common for sequence lengths and enum variant indices.
- **Optimized bulk byte reading**: Deserialization now uses slice splitting instead of byte-by-byte copying for integer parsing.
- **Added `#[inline]` hints** on hot serialization/deserialization paths for better performance.
- Replaced `sort_by` with `sort_unstable_by` for map key sorting (faster, no stability needed for unique keys).

### CI/CD

- Added separate CI jobs for formatting (`cargo fmt`), linting (`cargo clippy`), testing, coverage, documentation, and MSRV verification.
- Added code coverage reporting with Codecov integration and 90% line coverage threshold.
- Added Minimum Supported Rust Version (MSRV) check at Rust 1.78.0.
- Improved CI caching for faster builds.
- Documentation builds now use `-D warnings` to catch doc issues.

### Testing

- Expanded test suite with security-focused tests for memory amplification and duplicate key detection.
- Added tests for `to_bytes_with_capacity`, `from_bytes_seed`, `from_bytes_seed_prefix`, and other previously uncovered code paths.
- Improved benchmark suite with comprehensive type coverage and deserialization benchmarks.

## [v0.1.3] - 2021-05-18
- Generalized `serialize_into` to accept `W: ?Sized` writers.

## [v0.1.2] - 2020-12-14
- Improved README and documentation.

## [v0.1.1] - 2020-12-11
- Renaming crate into "bcs".

## [v0.1.0] - 2020-11-17
- Initial release.

[Unreleased]: https://github.com/aptos-labs/bcs/compare/v0.1.4...HEAD
[v0.1.4]: https://github.com/aptos-labs/bcs/compare/v0.1.3...v0.1.4
[v0.1.3]: https://github.com/aptos-labs/bcs/compare/v0.1.2...v0.1.3
[v0.1.2]: https://github.com/aptos-labs/bcs/compare/v0.1.1...v0.1.2
[v0.1.1]: https://github.com/diem/bcs/releases/tag/v0.1.1
[v0.1.0]: https://github.com/diem/bcs/releases/tag/v0.1.0
