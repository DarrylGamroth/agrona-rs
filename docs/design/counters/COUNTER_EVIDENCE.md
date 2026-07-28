# Counter-reader verification evidence

## Scope and baseline

- Requirements: `CTR-LAYOUT-001` through `CTR-PORT-001` in
  [`COUNTER_SPEC.md`](COUNTER_SPEC.md).
- Agrona Java behavioral baseline:
  `d4a47c67258f85b39910c4999da346ead655b736`.
- Aeron C layout and ordering comparison:
  `e44cd27a3b357c27ad37f6107a957f46d95552ac`.
- Verified implementation:
  `bc3e2ee471aedaa62c879f5dac01d191aa887bbe`.
- Local environment: Debian Linux
  `6.12.57+deb13-amd64`, x86_64, rustc 1.93.0, cargo 1.85.0, and
  OpenJDK 21.0.11.
- GitHub Actions:
  [run 30387109112](https://github.com/DarrylGamroth/agrona-rs/actions/runs/30387109112),
  successful after rerunning two jobs affected by the pre-existing
  `agent_allocation` test's process-global allocation-counter race.

## Requirement evidence

| Requirement | Implementation | Local evidence | State |
| --- | --- | --- | --- |
| `CTR-LAYOUT-001` | `counters_reader.rs` exact constants and offsets | Layout tests and pinned-Java fixture | Validated |
| `CTR-VALID-001` | `AlignedRegion`, constructor, checked offsets, and typed errors | Boundary, alignment, ID, and malformed-label tests | Validated |
| `CTR-READ-001` | Acquire value/registration reads; relaxed owner/reference reads | Direct-read, Java-interoperability, zero-allocation, and native matrix tests | Validated |
| `CTR-META-001` | Acquire state/label-length publication; borrowed key/label | Metadata and 4,096-record publication stress tests on x86_64 and AArch64 | Validated |
| `CTR-ITER-001` | Dense scans and first-match searches | Allocated/reclaimed/unused, first-UNUSED, and Java-fixture tests | Validated |
| `CTR-LIFE-001` | Borrowed reader plus confined checked unsafe region | Lifetime API, misalignment tests, unsafe review, and native matrix | Validated |
| `CTR-ALLOC-001` | Borrowed byte views and non-allocating scans | Counting-global-allocator test across 1,000 repetitions | Validated |
| `CTR-PORT-001` | Rust 2024, MSRV 1.85, native-atomic guard, existing native CI matrix | Rust 1.85 and native Linux/macOS/Windows x86_64/AArch64 CI | Validated |

`COUNTER-READER` is complete for this specified read-only increment.
`COUNTER-FAMILY` remains partial because manager allocation/reclamation,
`AtomicCounter`, and applicable position/status types are absent.

## Java interoperability evidence

`scripts/generate_counters_reader_java_fixture.py` builds the pinned Agrona
Java sources, including Agrona's generated unsafe-access implementation,
compiles `CountersReaderFixtureGenerator.java`, and uses the actual
`CountersManager` and `AtomicCounter` implementations to publish four records:
two allocated, one reclaimed, and the first unused record.

The generated native-endian files have these local hashes:

- `metadata.bin`:
  `ad59ea409825a5635fb91f41ae511e09a8cfa5cf773aabe127d0179acb4e0460`
- `values.bin`:
  `b71d2d3b42a32766a2577122aa7a1ded776822d7d8e019396c4266208f05d216`

`counters_reader_java_interop.rs` reads those Java-produced bytes through the
public Rust API and verifies every allocated field, the maximum key and label,
the reclaimed and unused states, enumeration, and both search forms. The
`java-counter-interop` CI job regenerated the files with Temurin Java 17,
byte-compared them with the checked-in fixtures, and ran the Rust
interoperability test against the regenerated directory successfully in run
30387109112.

This proves counter-region ABI interoperability on a matching native-endian
platform. It does not by itself prove live cross-process mapped-memory
correctness.

## Unsafe invariants

All library unsafe operations are confined to
`src/concurrent/aligned_region.rs`. Construction and every field access
establish the following conditions before converting bytes to atomic
references:

1. Non-empty bases are aligned to the maximum required field alignment and
   each requested field offset is naturally aligned.
2. The checked slice contains the complete field, so the access remains
   within initialized caller-supplied bytes for the region lifetime.
3. The immutable slice borrow keeps the allocation address stable and prevents
   safe Rust code from obtaining a mutable alias through the same owner.
4. Concurrent integral access is performed only through compatible atomic
   types. Callers must not use non-atomic aliases for those bytes while the
   reader can access them.
5. Borrowed key and label views cannot outlive the reader borrow; external
   writers must not mutate the viewed bytes while such a view is live.
6. A future owning or mapping wrapper must retain the regions for the reader
   lifetime and uphold the same initialization, aliasing, and concurrent
   access contract.

No mutex or reader-writer lock is present.

## Memory-order proof

Agrona publishes an allocated metadata record by release-storing
`RECORD_ALLOCATED`. The reader acquire-loads state before consuming type ID,
reuse deadline, key, or label data, creating the required synchronizes-with
edge when it observes that publication. Agrona's label update publishes the
new length with a release store; the reader acquire-loads label length before
borrowing the published prefix.

Counter value and registration ID use acquire loads as required by the
upstream volatile reader operations. Owner ID and reference ID are upstream
plain reads; Rust still requires atomic access when the storage can be
concurrent, so they use `Relaxed` loads. Type ID and reuse deadline likewise
use `Relaxed` integral loads after the acquired state. These relaxed
adaptations prevent data races without inventing an additional
synchronizes-with relationship. The implementation requires native 64-bit
atomics, so these accesses do not fall back to locks on supported targets.

The publication stress test initializes each record's numeric fields, key,
and label; release-publishes label length, value, registration ID, and state;
then verifies the complete record after the reader observes allocated state.
It passed locally on x86_64 and in CI on native Linux x86_64, Linux AArch64,
macOS AArch64, and Windows x86_64.

## Local verification results

All required acceptance commands completed successfully:

```text
cargo fmt --all --check
PASS

cargo test --workspace --all-targets --all-features
PASS

cargo test --workspace --all-features --doc
PASS (12 doctests and 3 compile-fail doctests)

cargo clippy --workspace --all-targets --all-features -- -D warnings
PASS

RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
PASS

cargo +1.85.0 check --workspace --all-targets --all-features
PASS
```

Additional local checks:

```text
Java fixture generation from pinned Agrona and Rust interop test
PASS

cargo llvm-cov --workspace --all-features --summary-only
PASS
TOTAL: 95.01% lines, 97.87% functions, 95.93% regions
CountersReader: 94.93% lines
CountersReaderError: 100.00% lines
AlignedRegion unsafe substrate: 100.00% lines

git diff --check
PASS
```

GitHub Actions run 30387109112 verified the exact implementation commit:

```text
Format, lint, and document
PASS

Check Rust 1.85 MSRV
PASS

Test stable on Linux x86_64
PASS

Test stable on Linux AArch64
PASS

Test stable on macOS AArch64
PASS

Test stable on Windows x86_64
PASS

Measure and upload coverage
PASS

Verify Agrona Java counter ABI with Java 17
PASS
```

The first executions of the Linux x86_64 and macOS AArch64 jobs stopped in the
unchanged `agent_allocation` test before reaching counter tests because its
process-global allocator observed unrelated test-harness allocations. Both
jobs passed on rerun; Linux AArch64 and Windows passed initially. No counter
code or test was changed to obtain the successful rerun.

## Intentional adaptations and residual gaps

The Rust API uses borrowed byte slices, checked construction, typed errors,
`Result<Option<i32>, CountersReaderError>` searches, and callbacks that can
propagate `CountersReaderError`. Labels remain bytes on the basic read path
rather than being decoded into allocating strings. Relaxed atomics adapt the
upstream plain integral reads to Rust's data-race rules without strengthening
their ordering contract.

No evidence in this document claims an Aeron CnC or other container, memory-map
creation or ownership, serialization, an application or RTC counter catalogue,
`CountersManager`, `AtomicCounter`, `ConcurrentCountersManager`,
position/status wrappers, or cross-process correctness. The next coherent
increment must add single-owner allocation/free, release publication, reuse
delay, registration-ID ABA protection, single-writer release operations, and
explicit multi-writer read-modify-write operations before those capabilities
can be claimed.
