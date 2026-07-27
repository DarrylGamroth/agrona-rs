# Clock verification evidence

## Scope and baseline

- Requirements: `CLK-DOM-001` through `CLK-PORT-002` in
  [`CLOCK_SPEC.md`](CLOCK_SPEC.md).
- Upstream behavioral baseline:
  `d4a47c67258f85b39910c4999da346ead655b736`.
- Verified Rust implementation:
  `11193e1fe9bc0a3e4beac42ac1d9b26fc43eaed2`.
- GitHub Actions:
  [run 30305348451](https://github.com/DarrylGamroth/agrona-rs/actions/runs/30305348451).
- Local environment: Debian Linux 6.12.57, x86_64, AMD Ryzen 7 6800H,
  `rustc 1.93.0 (254b59607 2026-01-19)`.
- MSRV check: Rust 1.85.0.

This document combines local x86_64 evidence with native GitHub-hosted x86_64
and AArch64 evidence for Linux, macOS, and Windows.

## Requirement coverage

| Requirement | Implementation | Verification |
|---|---|---|
| `CLK-DOM-001` | The four provider traits are each defined in their own component file and re-exported by `src/clock/mod.rs`. | Object-safety tests and compile-fail domain examples. |
| `CLK-SYS-001` | `src/clock/system_epoch_clock.rs`, `system_epoch_micro_clock.rs`, `system_epoch_nano_clock.rs`, and private conversion helpers in `system_time.rs`. | Zero-size, pre/post-epoch conversion, wrapping boundary, and current-time tests in separate component test files. |
| `CLK-MONO-001` | `src/clock/system_nano_clock.rs`. | Repeated ordering, elapsed-time, zero-size, and wrapping conversion tests. |
| `CLK-CACHE-001` | `src/clock/cached_epoch_clock.rs` and `cached_nano_clock.rs`. | Initial value, reader behavior, and non-cloneable-writer compile-fail examples. |
| `CLK-CACHE-002` | `src/clock/atomic_clock_value.rs` provides the shared cache-line-isolated atomic value. | Update, wrapping advance, 128-byte isolation, multi-reader publication, and ordering source review. |
| `CLK-OFFSET-001` | `src/clock/offset_epoch_nano_clock.rs`. | Defaults and every configuration boundary. |
| `CLK-OFFSET-002` | `src/clock/offset_epoch_nano_clock.rs`. | Scripted first-match, strict-threshold, midpoint, narrowest fallback, saturation, retry, and invalid-window tests. |
| `CLK-OFFSET-003` | One immutable `Sample` is published through `arc-swap` 1.9.2; normal reads use one `load_full`. | Coherent-publication stress, interval/backward resampling, concurrent readers and samplers, allocation test, progress/order audit, and native x86_64/AArch64 CI. |
| `CLK-OFFSET-004` | Failed automatic sampling retains the already-loaded immutable sample. | Scripted automatic failure plus explicit invalid-sample error tests. |
| `CLK-OFFSET-005` | Sampling measures independently and performs one lock-free replacement; no mutex, reader/writer lock, or poison state exists. | Overlapping sampler test, concurrent reader/sampler test, last-replacement behavior, and recovery after a panicking source. |
| `CLK-ALLOC-001` | All normal read paths and cached updates avoid heap creation. | Dedicated counting-allocator integration test after warmup, including `ArcSwap::load_full` reference-count operations. |
| `CLK-PORT-001` | Library source is safe Rust, uses `std`, and rejects targets without native 64-bit atomics. | Source inspection, stable Linux/macOS/Windows CI, and the Rust 1.85 check passed. |
| `CLK-PORT-002` | `.github/workflows/ci.yml` includes Linux x86_64, Linux AArch64, macOS AArch64, and Windows x86_64 native runners. | The concurrency suite passed natively on both x86_64 and AArch64. |

## Concurrency and memory-order audit

### Cached clocks

`AtomicClockValue` stores the complete `i64` clock value in one `AtomicI64`.
The unique writer obtains its previous value with a relaxed load only for its
own wrapping `advance`, then publishes the result with a release store.
Readers use acquire loads. The atomic value cannot tear, and release/acquire
establishes publication in the Rust memory model on both x86_64 and AArch64;
the implementation does not depend on x86's stronger hardware ordering.

### Offset clock

`OffsetEpochNanoClock` fully constructs an immutable `Sample` before
publication through `ArcSwap<Sample>`. The lockfile-selected `arc-swap` 1.9.2
implementation documents `load_full` as lock-free and wait-free, documents
writers as lock-free, and publishes by a sequentially consistent atomic
pointer swap. One returned `Arc<Sample>` keeps the exact snapshot alive while
all three fields are used. Consequently, a reader cannot combine fields from
different samples, and concurrent samplers do not serialize through a lock.

These guarantees classify the snapshot synchronization operations only.
Injected clock calls, allocation of a replacement `Arc`, the resampling
branch, and the complete `nano_time` operation are not claimed to be
wait-free. Native x86_64 and AArch64 tests supplement this source and memory
model review; they do not replace it.

## Commands and local results

The following commands pass for the working tree:

```text
cargo fmt --all --check
cargo test --workspace --all-targets --all-features
cargo test --workspace --all-features --doc
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features
cargo llvm-cov --workspace --all-features --summary-only
```

The local suite contains eight library tests, twenty-three integration tests
(including the dedicated allocation binary), one positive doctest, and three
compile-fail doctests. Component behavior tests are kept in separate files
under `tests/`, mirroring the one-public-component-per-file source layout.
Line coverage is 94.08% (287 instrumented lines, 17 missed). The
specification-workflow traceability checker matched all 13 ledger requirements
to 13 normative IDs with zero errors and zero warnings.

## GitHub CI results

GitHub Actions run `30305348451` passed at verified revision `11193e1`:

| Job | Result |
|---|---|
| Format, lint, and document | Passed |
| Test stable on Linux x86_64 | Passed |
| Test stable on Linux AArch64 | Passed |
| Test stable on macOS AArch64 | Passed |
| Test stable on Windows x86_64 | Passed |
| Check Rust 1.85 MSRV | Passed |
| Measure and upload coverage to Codecov | Passed |

## Benchmark baseline

Command, repeated three times:

```text
cargo bench --bench clocks
```

The release-profile benchmark performs 10,000 warmup iterations followed by
10,000,000 closed-loop iterations per operation. The table reports the median
and observed range across three runs:

| Operation | Median | Range |
|---|---:|---:|
| `SystemEpochClock::time` | 31.463 ns/op | 31.161–31.617 ns/op |
| `SystemEpochNanoClock::nano_time` | 31.404 ns/op | 30.548–31.780 ns/op |
| `SystemNanoClock::nano_time` | 24.223 ns/op | 23.642–24.352 ns/op |
| `CachedEpochClockReader::time` | 0.256 ns/op | 0.239–0.261 ns/op |
| `OffsetEpochNanoClock::nano_time` normal path | 9.907 ns/op | 9.746–9.936 ns/op |

This is a reproducibility baseline, not a latency contract or a cross-language
comparison. It measures neither contention nor resampling, reports aggregate
closed-loop service time rather than a latency distribution, and was run on a
shared development host with frequency boost enabled and without CPU affinity
or power-policy controls.

## Claim limits

The benchmark covers one Linux x86_64 environment only. No formal weak-memory
model checker is included; correctness rests on the Rust atomic contract, the
audited `arc-swap` contract, stress tests, and native x86_64/AArch64 CI.
Neither limitation is an unmet obligation in the active Clock specification.

The Clock implementation is complete against the active specification, and
the `CLOCK-READY` evidence gate is closed.
