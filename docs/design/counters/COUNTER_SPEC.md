# agrona-rs counter-reader specification

> Maintainer specification. The counter capability is partial.

Status: active

Baseline: Agrona Java commit
`d4a47c67258f85b39910c4999da346ead655b736` and Aeron C commit
`e44cd27a3b357c27ad37f6107a957f46d95552ac`

Applicable profile: Rust 2024 with Rust 1.85 or later on the maintained
`std` Linux, macOS, and Windows targets with native 64-bit atomics.

## Purpose and authority

This document is the normative Rust contract for `DEC-COUNTER-001` in the
[porting plan](../PORTING_PLAN.md). Agrona Java is the behavioral authority.
Aeron C is the native layout and ordering cross-check.

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

## Claim limits

Conformance claims byte compatibility only with the two Agrona/Aeron counter
regions on a matching native-endian platform and compatible atomic target.
It does not claim Java object or source compatibility, an Aeron CnC or other
container format, mapping creation, a cross-process allocator, manager
mutation, `AtomicCounter`, position/status wrappers, application type IDs,
type-specific key encoders, RTC counters, or cross-process correctness from a
single-architecture test.
