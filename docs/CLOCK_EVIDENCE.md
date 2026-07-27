# Clock verification evidence

## Scope and baseline

- Requirements: `CLK-DOM-001` through `CLK-PORT-001` in
  [`CLOCK_SPEC.md`](CLOCK_SPEC.md).
- Upstream behavioral baseline:
  `d4a47c67258f85b39910c4999da346ead655b736`.
- Rust implementation: current uncommitted agrona-rs working tree on
  2026-07-27.
- Local environment: Debian Linux 6.12.57, x86_64,
  `rustc 1.93.0 (254b59607 2026-01-19)`.
- MSRV check: Rust 1.85.0.

This evidence qualifies the local `std-linux` surface. The repository CI is
configured for Linux, macOS, and Windows, but macOS and Windows results require
the working tree to be committed and run by CI.

## Requirement coverage

| Requirement | Implementation | Verification |
|---|---|---|
| `CLK-DOM-001` | `src/clock/mod.rs` | Object-safe trait test plus compile-fail domain doctest. |
| `CLK-SYS-001` | `src/clock/system.rs` | Zero-size, pre/post-epoch conversion, wrapping boundary, and current-time tests. |
| `CLK-MONO-001` | `src/clock/system.rs` | Repeated ordering and elapsed-time tests. |
| `CLK-CACHE-001` | `src/clock/cached.rs` | Initial value, reader behavior, and non-cloneable-writer doctests. |
| `CLK-CACHE-002` | `src/clock/cached.rs` | Update, wrapping advance, cache alignment, and multi-reader publication tests. |
| `CLK-OFFSET-001` | `src/clock/offset.rs` | Defaults and every configuration boundary. |
| `CLK-OFFSET-002` | `src/clock/offset.rs` | Scripted first-match, strict-threshold, midpoint, narrowest fallback, saturation, retry, and invalid-window tests. |
| `CLK-OFFSET-003` | `src/clock/offset.rs` | Versioned-publication stress, interval/backward resampling, concurrent readers, and concurrent samplers. |
| `CLK-OFFSET-004` | `src/clock/offset.rs` | Scripted automatic failure retains the sample; explicit invalid sampling and poisoned-lock errors are tested. |
| `CLK-ALLOC-001` | all Clock modules | Dedicated counting-allocator integration test after warmup. |
| `CLK-PORT-001` | all Clock modules and CI | Library source contains no `unsafe`; Rust 1.85 check and local Linux tests pass. |

## Commands and results

The following commands passed:

```text
cargo fmt --all --check
cargo test --workspace --all-targets --all-features
cargo test --doc
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features
python3 check_traceability.py --ledger docs/clock_traceability.toml --root . --strict
```

The local suite contains five library tests, one allocation test, seventeen
behavior/concurrency tests, one positive doctest, and three compile-fail
doctests.

## Benchmark baseline

Command:

```text
cargo bench --bench clocks
```

Release-profile closed-loop results from the local environment, with
10,000,000 iterations per operation:

| Operation | Local result |
|---|---:|
| `SystemEpochClock::time` | 29.478 ns/op |
| `SystemEpochNanoClock::nano_time` | 29.435 ns/op |
| `SystemNanoClock::nano_time` | 22.319 ns/op |
| `CachedEpochClockReader::time` | 0.224 ns/op |
| `OffsetEpochNanoClock::nano_time` normal path | 3.630 ns/op |

These numbers are a reproducibility baseline, not a latency guarantee or a
cross-language comparison. The benchmark is closed-loop and does not measure
contention or resampling.

## Residual evidence gaps

- macOS and Windows tests have not run for this uncommitted working tree;
- no dedicated weak-memory model checker has been added; the versioned sample
  protocol currently has source review and stress evidence; and
- benchmark results cover one Linux x86_64 host only.

The Clock implementation is complete against the specified behavior, but the
`CLOCK-READY` evidence gate remains partial until the configured
multi-operating-system CI completes.
