# Counter-infrastructure implementation plan

## Baseline and scope

The delivery baseline is `COUNTER_SPEC.md` at the pinned Agrona Java and
Aeron C revisions recorded there. The existing `COUNTER-READER` gate remains
closed. This increment implements `CTR-MGR-001` through `CTR-INTEROP-001` and
keeps the wider `COUNTER-FAMILY` capability partial.

Selected components are `CountersManager`, `AtomicCounter`,
`ReadablePosition`, `Position`, `AtomicLongPosition`,
`UnsafeBufferPosition`, `StatusIndicatorReader`, `StatusIndicator`, and
`UnsafeBufferStatusIndicator`. `ConcurrentCountersManager`, mappings,
container formats, Aeron CnC parsing, and application counter catalogues are
excluded.

## Design allocation

One public Agrona component is kept in each correspondingly named Rust file
under `src/concurrent/status`. Rust-only typed errors have their own files.
`src/concurrent/aligned_region.rs` remains the only module that converts byte
addresses into atomic references.

`CountersManager` exclusively borrows both mutable regions, owns a high-water
mark and reusable-ID list, and is intentionally not thread-shareable. It uses
the values-derived capacity. Metadata byte mutation requires `&mut self`;
integral fields shared with readers are accessed atomically.

`AtomicCounter`, buffer positions, and status indicators borrow only the
values region and contain no manager reference. They can be shared for atomic
value operations. Closing a handle is local and idempotent; freeing its
registry record is an explicit manager operation. This is the safe Rust
adaptation of Java's optional manager callback.

## Allocation and lifecycle

Allocation first checks reusable IDs in Agrona free-list order and selects the
first whose signed deadline is not later than the supplied epoch clock. If no
ID is reusable, allocation extends the dense high-water mark. Payload fields
and label bytes are initialized before label length and state are
release-published.

Freeing release-publishes `RECLAIMED`, clears the complete key, writes the
wrapping reuse deadline, and queues the ID. Reuse release-resets value and
registration ID, relaxed-resets owner and reference IDs, then initializes and
publishes the new metadata record. Invalid and double frees are errors.

## Ordering proof

| Agrona access | Rust ordering |
|---|---|
| volatile read/write and multi-writer RMW | `SeqCst` |
| acquire read | `Acquire` |
| release/ordered write | `Release` |
| plain or opaque integral access | `Relaxed` |

Rust has no Java opaque ordering and safe Rust cannot express a concurrent
racy plain access. `Relaxed` therefore preserves atomicity without adding a
synchronizes-with edge. Single-writer release, opaque, plain, and propose-max
updates remain load-then-store operations rather than RMW operations, so their
upstream lost-update classification is preserved.

Metadata publication is payload bytes followed by release `ALLOCATED` state.
The string-style allocation and label-mutation paths release-publish label
length. Agrona's copied-buffer allocation writes label length plainly before
state publication; `allocate_raw` preserves that distinction with a relaxed
length store whose visibility is supplied by the later state release.
Readers acquire state before consuming payload and acquire label length
before borrowing label bytes. Counter values and registration IDs use
matching acquire/release operations. These relationships are valid on both
x86_64 and AArch64 and do not depend on x86 store ordering.

Signed counter arithmetic and reuse deadlines use wrapping operations where
Java arithmetic wraps.

## Unsafe invariants

The checked region module establishes:

1. storage is initialized, address-stable, naturally aligned, and valid for
   the complete borrowed lifetime;
2. each field access is bounds checked and naturally aligned;
3. mutable metadata access remains under the manager's unique borrow;
4. values-region integral access is exclusively atomic while shareable
   handles exist;
5. raw views cannot outlive their backing borrow;
6. key or label bytes are not mutated while a borrowed view to those bytes is
   live; and
7. the caller owns mapping lifetime and external-process coordination.

No raw-pointer arithmetic escapes this module.

## Delivery coverage

| Requirements | Implementation | Verification |
|---|---|---|
| `CTR-LAYOUT-001`–`CTR-ALLOC-001` | `CountersReader` and checked regions | existing reader tests and Java fixture |
| `CTR-MGR-001`, `CTR-ALLOC-002`, `CTR-REUSE-001`, `CTR-MUTATE-001` | `counters_manager.rs` and typed errors | manager construction, allocation, free/reuse, mutation, and publication tests |
| `CTR-ATOM-001`, `CTR-ATOM-LIFE-001` | `atomic_counter.rs` | operation, ordering, sharing, lifecycle, wrapping, and allocation tests |
| `CTR-POS-001` | position traits and `atomic_long_position.rs` | trait, ordering, propose-max, close, and allocation tests |
| `CTR-BUF-POS-001` | `unsafe_buffer_position.rs` | offset, validation, ordering, and publication tests |
| `CTR-STATUS-001` | status traits and `unsafe_buffer_status_indicator.rs` | offset, validation, ordering, and publication tests |
| `CTR-INTEROP-001` | Java fixture generator/validator and Rust interop test | Java-to-Rust and Rust-to-Java byte-region validation |
| `CTR-PORT-001` | native atomic guard and CI matrix | MSRV plus native x86_64/AArch64 CI |

## Capability gate

The selected requirements closed after local acceptance and native CI
evidence. `COUNTER-FAMILY` remains partial because
`ConcurrentCountersManager` is deliberately deferred and later
manager/position variants have not been reviewed. No RTC container, Aeron CnC
parser, mapping owner, or application counter catalogue is claimed.
