# Counter-infrastructure verification evidence

## Scope and baseline

- Requirements: all 18 `CTR-*` requirements in
  [`COUNTER_SPEC.md`](COUNTER_SPEC.md).
- Agrona Java behavioral baseline:
  `d4a47c67258f85b39910c4999da346ead655b736`.
- Aeron C layout and ordering comparison:
  `e44cd27a3b357c27ad37f6107a957f46d95552ac`.
- Reader baseline commit:
  `bc3e2ee471aedaa62c879f5dac01d191aa887bbe`.
- Mutable implementation commit:
  `e64f39ff274d5e50a5f71a94f4fc02d4fc54ad59`.
- Delivered `main` revision:
  `016cd2c96f7187dac744a31a82c17604b7ae018b`.
- Local environment: Linux `6.12.57+deb13-amd64`, x86_64, rustc/cargo
  1.93.0, cargo 1.85.0, and OpenJDK 21.0.11 compiling the fixture with
  `javac --release 17`.

The delivered reader and mutable counter family were verified together by
[GitHub Actions run 30394389866](https://github.com/DarrylGamroth/agrona-rs/actions/runs/30394389866)
on native Linux x86_64/AArch64, macOS AArch64, and Windows x86_64, including
the bidirectional Java ABI job.

## Requirement evidence

| Requirement | Implementation | Verification evidence | State |
| --- | --- | --- | --- |
| `CTR-LAYOUT-001`–`CTR-ALLOC-001` | `CountersReader` and checked aligned region | Existing reader tests, allocation test, Java fixture, and prior native CI | Validated |
| `CTR-MGR-001` | Single-owner `CountersManager` with checked mutable regions, values-derived capacity, free list, clock, and no lock | Empty/malformed/misaligned, capacity, availability, exhaustion, and native matrix tests | Validated |
| `CTR-ALLOC-002` | Dense high-water allocation, eligible reuse first, key/label initialization, rollback, release state publication | Allocation, failure recovery, exact metadata, truncation, publication, Java, and native tests | Validated |
| `CTR-REUSE-001` | Release reclaim, key clear, wrapping deadline, cooldown, value/identity resets | Invalid/double free, deadline boundary/wrap, reuse ordering, stale handle, interop, and native tests | Validated |
| `CTR-MUTATE-001` | Ordered value/identity mutation plus bounded key/label updates | Mutation, remainder preservation, truncation, append, malformed length, reader, and native tests | Validated |
| `CTR-ATOM-001` | Complete `AtomicCounter` ordering and operation families | Every operation/alias, wrapping, four-writer exact-count, publication, allocation, and native tests | Validated |
| `CTR-ATOM-LIFE-001` | Borrowed shareable handle, atomic local close, explicit manager free | Sharing, idempotent close, explicit free, reuse, stale-handle, guide, and native tests | Validated |
| `CTR-POS-001` | `ReadablePosition`, `Position`, and `AtomicLongPosition` | Trait, constructor, ordering, propose-max, close, publication, allocation, and native tests | Validated |
| `CTR-BUF-POS-001` | `UnsafeBufferPosition` over checked counter handles | Multiple ID/stride, validation, ordering, close, publication, allocation, and native tests | Validated |
| `CTR-STATUS-001` | Status traits and `UnsafeBufferStatusIndicator` | Multiple ID/stride, validation, ordering, publication, allocation, and native tests | Validated |
| `CTR-INTEROP-001` | Java generator/validator, Rust producer/consumer, and CI job | Pinned Java-to-Rust and Rust-to-Java procedure passed locally and in CI | Validated |
| `CTR-PORT-001` | Rust 2024, Rust 1.85, native-atomic guard, CI matrix | MSRV plus stable native Linux/macOS/Windows x86_64/AArch64 CI | Validated |

## Java interoperability

`scripts/generate_counters_reader_java_fixture.py` builds the pinned Agrona
Java sources and compiles `CountersReaderFixtureGenerator.java`. With
`--bidirectional` it performs both directions:

1. Agrona Java `CountersManager` produces allocated records, a maximum key and
   label, a freed-and-reused record with reset value/identity, a reclaimed
   record with cleared key, and the first unused record. Rust reads and checks
   every state and field.
2. Rust `CountersManager` produces allocated, maximum-key/label,
   freed-and-reused, and reclaimed records. The actual pinned Java
   `CountersReader` validates state, offsets, values, identities, reset
   behavior, label, key, and deadline.

The regenerated checked-in Java fixtures have these hashes:

- `metadata.bin`:
  `50a02463ed15d9763688c638a0041f0fbb29fd266b0bc4b7d5da370237d10c43`
- `values.bin`:
  `5db49348d5af29b4a0bd04a004f699107bf3bd8def1c964e587826ddf5576410`

The complete bidirectional command passed locally and in run 30394389866.
The CI job uses Temurin Java 17, regenerates and byte-compares the stable Java
fixtures, runs the Rust consumer/producer tests, then runs the Java validator.
This establishes region ABI interoperability on a matching native-endian
platform. It does not establish live cross-process mapped-memory correctness.

## Unsafe invariants

All library unsafe operations remain confined to
`src/concurrent/aligned_region.rs`.

1. Non-empty bases are validated to eight-byte alignment, and every field
   range is bounds checked and naturally aligned before atomic conversion.
2. Backing bytes are initialized, address-stable, and valid for the complete
   borrowed lifetime.
3. Read-only regions expose only immutable byte views to callers. Writable
   handle constructors require a mutable slice or a checked manager-produced
   atomic view; they never write through an ordinary public shared slice.
4. Manager metadata byte mutation requires its unique mutable borrow. A
   reader borrow prevents concurrent safe key or label mutation.
5. Values that can be shared are accessed only through compatible atomics;
   safe APIs do not expose a simultaneous non-atomic mutable alias.
6. `AlignedRegion` is `Send + Sync` because its shared operations are atomic
   or immutable. `MutableAlignedRegion` is `Send` but deliberately not `Sync`,
   preserving single-owner registry mutation.
7. A later mapping owner must retain storage and uphold initialization,
   lifetime, atomic-access, and external-producer coordination invariants.

No mutex or reader-writer lock is present.

## Memory-order proof

The manager writes type, deadline, key, and label payload before
release-storing `ALLOCATED`. Readers acquire-load state before consuming the
record. String-style allocation and every label mutation release-publish
label length. Agrona's copied-buffer allocation writes its length plainly
before state publication; Rust `allocate_raw` uses a relaxed length store
followed by the same release state publication.

Free release-stores `RECLAIMED` before clearing key bytes and writing the
reuse deadline, matching Agrona. Readers skip reclaimed records. Before
reallocation the manager release-resets registration ID and value, then
relaxed-resets owner and reference ID, before publishing the replacement
metadata state.

`AtomicCounter`, position, and status volatile operations use `SeqCst`.
Multi-writer fetch-add, swap, and compare-exchange are `SeqCst` RMW
operations. Acquire/release operations map directly. Java plain and opaque
integral accesses use `Relaxed`, which supplies atomicity but no
synchronizes-with edge. Single-writer add, increment, decrement, and
propose-max variants remain a relaxed load followed by the named store, not
an RMW, preserving their lost-update behavior. Signed arithmetic uses
wrapping operations.

These relationships use the Rust memory model and native 64-bit atomics and
do not depend on x86 store ordering. Publication and concurrency tests passed
locally on x86_64 and in CI on native Linux x86_64/AArch64, macOS AArch64,
and Windows x86_64.

## Local verification results

All required acceptance commands completed successfully:

```text
cargo fmt --all --check
PASS

cargo test --workspace --all-targets --all-features
PASS

cargo test --workspace --all-features --doc
PASS (12 doctests and 4 compile-fail doctests)

cargo clippy --workspace --all-targets --all-features -- -D warnings
PASS

RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
PASS

cargo +1.85.0 check --workspace --all-targets --all-features
PASS
```

Additional checks:

```text
Pinned Agrona Java-to-Rust and Rust-to-Java interop
PASS

Strict traceability checker: 18 requirements, 0 errors, 0 warnings
PASS
```

GitHub Actions run 30394389866 passed at delivered revision `016cd2c`:

```text
Format, lint, and document
PASS

Measure and upload coverage
PASS

Verify Agrona Java counter ABI
PASS

Test stable on Linux x86_64
PASS

Test stable on Linux AArch64
PASS

Test stable on macOS AArch64
PASS

Test stable on Windows x86_64
PASS

Check Rust 1.85 MSRV
PASS
```

## Intentional Rust adaptations and residual gaps

- Labels and keys are bytes in the non-allocating base API; string decoding
  remains caller-owned.
- Constructors and operations use typed `Result` errors. A fallible key
  initializer reports `CounterAllocationError<E>` and returns its selected ID
  to the free list.
- Writable direct handles require an exclusive mutable slice. Additional
  shareable handles are produced from the checked single-owner manager.
- Java abstract position/status classes become object-safe Rust traits.
- Java opaque and plain atomics map to Rust `Relaxed`.
- Closing a handle is local and atomic. Registry free is explicit because a
  shareable handle cannot safely retain and mutate the single-owner manager.
  A stale handle continues to address a reused slot, so applications must
  quiesce handles and use registration IDs for lifecycle identity.
- Like Agrona Java, `CountersManager` starts with high-water mark `-1` and
  does not reconstruct allocation state from pre-existing records. Give a new
  manager freshly initialized regions unless the allocation state is managed
  by the same live manager.

`ConcurrentCountersManager` remains deliberately deferred. It is an
intra-process lock around registry operations, not a cross-process allocator,
and adding it would require a separate selection decision. No evidence here
claims an Aeron CnC or RTC container, mapping creation or ownership, an
application counter catalogue, component-specific type IDs or key encoders,
serialization, or general cross-process correctness.
