# Counter-reader verification evidence

## Scope and baseline

- Requirements: `CTR-LAYOUT-001` through `CTR-PORT-001` in
  [`COUNTER_SPEC.md`](COUNTER_SPEC.md).
- Agrona Java behavioral baseline:
  `d4a47c67258f85b39910c4999da346ead655b736`.
- Aeron C layout and ordering comparison:
  `e44cd27a3b357c27ad37f6107a957f46d95552ac`.
- Candidate implementation: working tree based on
  `fa5a827d2232346f05f300f01f0dacc492b6f008`.
- Local environment: Debian Linux
  `6.12.57+deb13-amd64`, x86_64, rustc 1.93.0, cargo 1.85.0, and
  OpenJDK 21.0.11.
- GitHub Actions evidence for the delivered revision: pending.

## Requirement evidence

| Requirement | Implementation | Local evidence | State |
| --- | --- | --- | --- |
| `CTR-LAYOUT-001` | `counters_reader.rs` exact constants and offsets | Layout tests and pinned-Java fixture | Partial pending maintained-platform CI |
| `CTR-VALID-001` | `AlignedRegion`, constructor, checked offsets, and typed errors | Boundary, alignment, ID, and malformed-label tests | Partial pending maintained-platform CI |
| `CTR-READ-001` | Acquire value/registration reads; relaxed owner/reference reads | Direct-read, Java-interoperability, and zero-allocation tests | Partial pending native AArch64 CI |
| `CTR-META-001` | Acquire state/label-length publication; borrowed key/label | Metadata and 4,096-record publication stress tests | Partial pending native AArch64 CI |
| `CTR-ITER-001` | Dense scans and first-match searches | Allocated/reclaimed/unused, first-UNUSED, and Java-fixture tests | Partial pending maintained-platform CI |
| `CTR-LIFE-001` | Borrowed reader plus confined checked unsafe region | Lifetime API, misalignment tests, and unsafe review below | Partial pending delivered-revision CI |
| `CTR-ALLOC-001` | Borrowed byte views and non-allocating scans | Counting-global-allocator test across 1,000 repetitions | Partial pending maintained-platform CI |
| `CTR-PORT-001` | Rust 2024, MSRV 1.85, native-atomic guard, existing native CI matrix | Local MSRV check and x86_64 verification | Partial pending native AArch64/macOS/Windows CI |

Both `COUNTER-READER` and `COUNTER-FAMILY` remain partial. The reader gate can
be promoted only after the exact delivered revision passes its native CI
matrix. The family gate remains partial after that promotion because manager
allocation/reclamation, `AtomicCounter`, and applicable position/status types
are absent.

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
`java-counter-interop` CI job regenerates the files with Temurin Java 17,
byte-compares them with the checked-in fixtures, and runs the Rust
interoperability test against the regenerated directory.

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
It passed locally on x86_64. Native AArch64 execution is intentionally pending
CI evidence.

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
TOTAL: 92.67% lines, 97.34% functions, 93.21% regions
CountersReader: 94.93% lines
AlignedRegion unsafe substrate: 100.00% lines

git diff --check
PASS
```

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
