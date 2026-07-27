# Repository guidance for coding agents

## Authority

Agrona Java is the normative behavioral reference:
https://github.com/aeron-io/agrona/tree/d4a47c67258f85b39910c4999da346ead655b736

Aeron C is a secondary native implementation reference for Agent ownership,
thread lifecycle, atomic stop publication, and idle primitives:
https://github.com/aeron-io/aeron/tree/e44cd27a3b357c27ad37f6107a957f46d95552ac

Language-specific ports may be useful examples, but they are not behavioral
authorities or test oracles.

## Implementation rules

- Do not diverge from observable Agrona Java behavior where compatibility is
  claimed.
- Preserve progress guarantees. Never replace a lock-free upstream path with
  a mutex, reader-writer lock, or other blocking synchronization.
- Audit atomic ordering for both x86_64 and AArch64 memory models.
- Keep one public Agrona component per correspondingly named Rust source file.
- Keep public component tests in separate test files. Private unit tests are
  appropriate only when internal state must be inspected directly.
- Prefer idiomatic Rust ownership, traits, and typed errors where Java
  mechanics have no direct safe equivalent, and document each adaptation.
- Treat shared-memory facilities and `DynamicCompositeAgent` as deferred.
- Do not use the sibling Julia ports as sources of edge behavior.
- Retain applicable copyright and SPDX headers.

## Validation

Before submitting a change, run:

```text
cargo fmt --all --check
cargo test --workspace --all-targets --all-features
cargo test --workspace --all-features --doc
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features
```

Concurrency changes also require source-level ordering review, native x86_64
and AArch64 CI evidence, and focused liveness or publication tests.
Steady-state hot paths require allocation evidence.

## Releases

Releases are GitHub-only. Never publish this crate to crates.io. Do not create
or move a release tag until the exact candidate has passed the complete CI
matrix.
