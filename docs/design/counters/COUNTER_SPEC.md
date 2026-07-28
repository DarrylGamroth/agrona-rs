# agrona-rs counter infrastructure specification

> Maintainer specification. The counter capability excludes the explicitly
> deferred concurrent registry wrapper.

Status: active

Baseline: Agrona Java commit
`d4a47c67258f85b39910c4999da346ead655b736` and Aeron C commit
`e44cd27a3b357c27ad37f6107a957f46d95552ac`

Applicable profile: Rust 2024 with Rust 1.85 or later on the maintained
`std` Linux, macOS, and Windows targets with native 64-bit atomics.

## Purpose and authority

This document is the normative Rust contract for `DEC-COUNTER-001` and its
selected manager, atomic-counter, position, and status-indicator continuation
in the [porting plan](../PORTING_PLAN.md). Agrona Java is the behavioral
authority. Aeron C is the native layout and ordering cross-check.

Uppercase requirement terms use the meanings defined by BCP 14 (RFC 2119 and
RFC 8174).

## Requirements

### CTR-LAYOUT-001 — Exact values and metadata ABI

`CountersReader` MUST expose the Agrona counter constants; interpret
native-endian values records with a 128-byte stride containing signed 64-bit
value, registration ID, owner ID, and reference ID fields at offsets 0, 8, 16,
and 24 with padding through byte 127; interpret metadata records with a
512-byte stride containing signed 32-bit state and type ID fields at offsets 0
and 4, a signed 64-bit reuse deadline at 8, 112 key bytes at 16, a signed
32-bit label length at 128, and 380 label bytes at 132; define record states as
`UNUSED = 0`, `ALLOCATED = 1`, and `RECLAIMED = -1`; derive capacity and
maximum counter ID from the values region; and require the metadata region to
be at least four times the values-region length.

Verification intent: constant, stride, capacity, offset, empty, and maximum
representable fixture tests plus comparison with the pinned Java and C
definitions.

### CTR-VALID-001 — Bounds and alignment validation

Construction and access MUST reject non-empty metadata or values regions whose
base is not naturally aligned for every atomic field, partial values or
metadata records, metadata shorter than four times the values region,
capacities whose counter IDs cannot be represented by signed 32-bit IDs,
negative or out-of-capacity IDs before memory access, and label lengths
outside `0..=380` before forming a byte view.

Verification intent: zero, one, and maximum practical capacities;
insufficient and partial regions; deliberately misaligned bases; negative and
out-of-range IDs; arithmetic boundaries; and malformed label lengths.

### CTR-READ-001 — Value and identity reads

Counter identity access MUST use acquire loads for counter value and
registration ID; preserve upstream plain ordering for owner ID and reference
ID through Rust `Relaxed` atomic loads, which provide the weakest Rust atomic
ordering and no synchronizes-with edge; and keep repeated value reads bounded,
lock-free on the supported native-atomic profile, and free of heap allocation.

Verification intent: direct reads, registration searches, allocation
instrumentation, source-level ordering review, and native x86_64/AArch64 CI.

### CTR-META-001 — Metadata publication and access

Metadata access MUST acquire-load record state before consuming the
corresponding type ID, reuse deadline, key, label length, or label bytes;
acquire-load label length so a producer's release publication makes the
published label prefix visible; use `Relaxed` integral loads for type ID and
reuse deadline after state acquisition; borrow exactly 112 key bytes; and
borrow the validated published label prefix without allocation or decoding.

Verification intent: direct access, maximum key and label, malformed label,
and release/acquire publication stress tests plus source review.

### CTR-ITER-001 — Dense enumeration and search

Enumeration and search MUST visit only `ALLOCATED` records, skip `RECLAIMED`
records, stop at the first `UNUSED` record, never scan beyond the
values-derived capacity, enumerate in increasing counter-ID order, and return
the first registration-ID or type-ID-plus-registration-ID match or `None`.

Verification intent: allocated/reclaimed/unused permutations,
first-unused termination, value/metadata callback contents, and search tests.

### CTR-LIFE-001 — Borrowed region lifetime and aliasing safety

`CountersReader` MUST borrow rather than own its two regions without outliving
them or retaining a mutable reference; confine unsafe pointer conversion to
the checked region module; document invariants for base alignment, field
alignment, initialized bytes, stable address and lifetime, Rust aliasing, and
compatible atomic concurrent access; bind borrowed key and label views to the
reader borrow; forbid mutation through non-atomic aliases while the reader can
access the bytes; and forbid mutation of key or label bytes while a returned
borrowed view is live. A later owning allocation or mapping wrapper can
construct the same reader over its retained regions; this requirement does not
create or own a mapping.

Verification intent: compile-time lifetimes, construction tests, public API
inspection, and complete unsafe-invariant review.

### CTR-ALLOC-001 — Zero-allocation repeated reads

After construction, steady-state reader operations MUST perform zero heap
allocations for counter value, identity, state, type, deadline, key, label,
enumeration, and search unless caller-provided callback code allocates.

Verification intent: a dedicated integration-test binary with a counting
global allocator after warmup.

### CTR-PORT-001 — Supported platform profile

The supported profile MUST compile with Rust 1.85 in Rust 2024 edition, require
native 64-bit atomics, keep its tests enabled on maintained Linux, macOS, and
Windows CI, and execute publication tests natively on both x86_64 and AArch64
before validating cross-architecture evidence.

Verification intent: MSRV, stable, three-OS, x86_64, and AArch64 CI plus
source inspection.

### CTR-MGR-001 — Single-owner manager construction and capacity

`CountersManager` MUST borrow checked, aligned mutable metadata and values
regions; use the values-derived capacity and dense signed counter-ID space;
report capacity and currently allocatable slots; retain a caller-supplied
epoch-millisecond clock and signed reuse timeout; own its high-water
mark and free list without a mutex or reader-writer lock; and reject malformed
regions before mutation.

Verification intent: empty, one-record, full-capacity, malformed, misaligned,
capacity, and available-count tests plus source inspection.

### CTR-ALLOC-002 — Counter allocation and publication

The single-owner manager MUST allocate reusable eligible IDs before extending
the dense high-water mark; reject allocation when full; initialize type,
deadline, optional key, and truncated label before release-publishing
`ALLOCATED`; preserve zero-filled unused key bytes; return an acquired ID to
the free list if key initialization fails; and leave counter value and
identity defaults compatible with Agrona.

Verification intent: sequential allocation, exhaustion, label/key bounds,
initializer failure and recovery, exact metadata bytes, first-UNUSED
enumeration, and Java/Rust interoperability tests.

### CTR-REUSE-001 — Reclamation, cooldown, and ABA identity reset

Free and reuse MUST reject invalid or non-allocated IDs; release-publish
`RECLAIMED` before clearing all 112 key bytes; store the wrapping
epoch-millisecond reuse deadline; withhold the ID until `now >= deadline`;
and, before reallocation, release-reset value and registration ID while
relaxed-resetting owner and reference IDs so registration identity can
distinguish reuse.

Verification intent: double-free, invalid ID, key clearing, timeout boundary,
available count, wrapping deadline, reuse order, and all value/identity reset
tests.

### CTR-MUTATE-001 — Manager metadata and identity mutation

Manager mutation MUST release-store counter value and registration ID;
relaxed-store owner and reference IDs; reject oversized replacement keys;
copy accepted key prefixes without modifying the remainder; truncate labels
to 380 bytes; acquire the published label length before append; and
release-publish replacement or appended label length after writing label
bytes.

Verification intent: direct reader observation, key boundary and remainder,
label truncation and append-at-capacity, and release/acquire publication
tests.

### CTR-ATOM-001 — AtomicCounter operations and ordering

`AtomicCounter` MUST validate its values-region record; use sequentially
consistent loads, stores, fetch-add, swap, and compare-exchange for upstream
volatile or multi-writer operations; use acquire loads and release stores for
the corresponding ordered operations; adapt upstream plain and opaque
operations to relaxed atomics; retain the upstream single-writer
load-then-store behavior for release, opaque, plain, and propose-max
operations; use signed wrapping arithmetic; and allocate zero bytes on every
steady-state value operation.

Verification intent: every operation and alias, wrapping boundaries,
multi-writer exact-count stress, release/acquire publication, lost-update
classification by source review, and counting-allocator tests.

### CTR-ATOM-LIFE-001 — Rust counter-handle lifecycle

Counter handles MUST borrow caller-owned values storage for their lifetime,
remain safe to share for atomic value operations, track idempotent local close
state atomically, and require explicit `CountersManager::free` for registry
reclamation rather than retaining a non-thread-safe manager reference; direct
writable construction requires an exclusive mutable region, while additional
shareable handles come from the manager's checked atomic view.

Verification intent: compile-time sharing, close idempotence, storage
lifetime, stale-handle/reuse behavior, and explicit-free documentation tests.

### CTR-POS-001 — Process-local position contract

`ReadablePosition`, `Position`, and `AtomicLongPosition` MUST expose Agrona's
ID, close, read, write, and propose-max behavior; map volatile operations to
sequential consistency, acquire/release operations directly, and
plain/opaque operations to relaxed atomics; keep propose-max single-writer;
and use zero-allocation steady-state operations.

Verification intent: trait use, constructor defaults, every ordering family,
propose-max boundaries, close state, cross-thread publication, and allocation
tests.

### CTR-BUF-POS-001 — Counter-buffer position contract

`UnsafeBufferPosition` MUST address the counter value at the exact
128-byte-stride ABI offset; validate region alignment, boundaries, and ID;
implement all position ordering and propose-max behavior through compatible
atomics; track local close state; and leave manager reclamation explicit.
Direct writable construction takes exclusive mutable storage; wrapping a
manager-produced checked counter handle is the safe shared-region path.

Verification intent: multiple IDs and offsets, invalid construction, every
ordering family, release/acquire publication, close behavior, and Java-layout
fixture tests.

### CTR-STATUS-001 — Counter-buffer status indicator contract

`StatusIndicatorReader`, `StatusIndicator`, and
`UnsafeBufferStatusIndicator` MUST address the counter value at the exact ABI
offset; validate construction; provide volatile, acquire, and opaque reads
and volatile, release, ordered-alias, and opaque writes using the Rust
ordering adaptations defined above; and allocate zero bytes in steady state.
Its writable construction follows the same exclusive-region or
manager-produced-handle rule as buffer positions.

Verification intent: trait use, multiple IDs, invalid construction, every
ordering family, cross-thread publication, and allocation tests.

### CTR-INTEROP-001 — Bidirectional Java region interoperability

The maintained interoperability procedure MUST generate allocated,
reclaimed, reused, maximum-key, and maximum-label records with the pinned
Agrona Java manager for Rust validation and generate equivalent regions with
the Rust manager for validation by the pinned Java reader on a matching
native-endian platform.

Verification intent: Java 17 CI builds pinned Agrona, performs both
producer/consumer directions, and byte-checks stable Java-produced fixtures.

## Claim limits

Conformance claims byte compatibility only with the two Agrona/Aeron counter
regions on a matching native-endian platform and compatible atomic target.
It does not claim Java object or source compatibility, an Aeron CnC or other
container format, mapping creation, a cross-process allocator,
`ConcurrentCountersManager`, application type IDs, type-specific key
encoders, RTC counters, or cross-process correctness from a
single-architecture test.

Automatic free-on-close through a retained manager reference is intentionally
adapted to explicit manager reclamation. This keeps atomic counter handles
thread-shareable without making the single-owner manager registry
concurrently mutable or adding a lock.
