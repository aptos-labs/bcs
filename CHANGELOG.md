# Changelog

## [Unreleased]

### Security

- **Memory amplification protection**: Deserialization now validates that claimed sequence/string lengths are plausible given the remaining input, preventing DoS attacks where a small malicious payload could trigger large memory allocations.
- **Duplicate map key detection**: Serialization now returns `Error::NonCanonicalMap` when duplicate keys are encountered instead of silently dropping duplicates, ensuring data integrity.

### Added

- `to_bytes_with_capacity()` function for pre-allocating output buffers when the serialized size is known or estimated, reducing allocations.
- Comprehensive `# Errors` documentation sections on all public functions.
- `#[must_use]` attribute on `is_human_readable()`.
- Explicit `#![forbid(unsafe_code)]` via Cargo.toml lints section.
- Full pedantic clippy lint compliance with minimal, justified exceptions for binary serialization casts.
- `rustfmt.toml` configuration for consistent code formatting.

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
- Added tests for `to_bytes_with_capacity`, `from_bytes_seed`, and other previously uncovered code paths.
- Improved benchmark suite with comprehensive type coverage and deserialization benchmarks.

## [v0.1.1] - 2020-12-11
- Renaming crate into "bcs".

## [v0.1.0] - 2020-11-17
- Initial release.

[Unreleased]: https://github.com/diem/bcs/compare/v0.1.1...HEAD
[v0.1.1]: https://github.com/diem/bcs/releases/tag/v0.1.1
[v0.1.0]: https://github.com/diem/bcs/releases/tag/v0.1.0
