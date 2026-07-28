# Counter-reader implementation plan

## Baseline and scope

The delivery baseline is `COUNTER_SPEC.md` at the Agrona Java and Aeron C
revisions recorded there. The in-scope requirements are `CTR-LAYOUT-001`,
`CTR-VALID-001`, `CTR-READ-001`, `CTR-META-001`, `CTR-ITER-001`,
`CTR-LIFE-001`, `CTR-ALLOC-001`, and `CTR-PORT-001`.

This increment implements only a read-only `CountersReader` over two borrowed
regions. Manager allocation/free, counter mutation, positions/status
wrappers, mappings, container formats, and application catalogues are
excluded.

## Design allocation

`src/concurrent/status/counters_reader.rs` owns the public constants,
construction, access, enumeration, and search contract.
`src/concurrent/status/counters_reader_error.rs` owns typed construction and
access errors. `src/concurrent/aligned_region.rs` is the only library module
that converts byte addresses to atomic references.

The public constructor accepts metadata then values byte slices, matching
Agrona's constructor order. Both slices remain caller-owned. A later owning
buffer or mapping can expose its retained regions to this constructor.

The values record count is the reader capacity. Scans use that bound even if
metadata has extra complete records; this is the safe Rust interpretation of
Agrona's values-derived `maxCounterId`.

## Ordering proof

The future single-owner manager initializes metadata payload and label bytes,
release-publishes label length, then release-publishes `ALLOCATED` state.
Every metadata scan acquire-loads state before type, deadline, key, or label.
Label access then acquire-loads label length before borrowing the published
prefix. Either acquire pairs with the corresponding release on x86_64 and
AArch64; no x86-only store-order assumption is used.

Counter value and registration ID pair with upstream release or atomic
writers through acquire loads. Owner ID, reference ID, type ID, and reuse
deadline have upstream plain semantics. Rust uses `Relaxed` atomics for these
fields because a concurrent plain load would be undefined; `Relaxed` adds
atomicity and modification-order participation but no publication edge.

All record offsets are naturally aligned because bases are validated to 8
bytes and record strides and field offsets are multiples of the operand
alignment.

## Unsafe invariants

The private aligned-region module requires:

1. the borrowed slice address remains stable and valid for its lifetime;
2. all bytes are initialized;
3. the base is 8-byte aligned before any atomic conversion;
4. every integral field offset is bounds-checked and naturally aligned;
5. same-process concurrent writers use compatible atomic access for integral
   fields and do not mix racy non-atomic access;
6. no same-process writer mutates key or label bytes while a borrowed view is
   live; and
7. the storage owner, not the reader, controls mapping lifetime and external
   producer coordination.

## Delivery coverage

| Requirement | Implementation | Verification |
|---|---|---|
| `CTR-LAYOUT-001` | constants and offset functions in `counters_reader.rs` | `tests/counters_reader_layout.rs` |
| `CTR-VALID-001` | checked constructor, ID/label checks, aligned region | `tests/counters_reader_validation.rs` |
| `CTR-READ-001` | acquire and relaxed integral reads | `tests/counters_reader.rs`, allocation test |
| `CTR-META-001` | state/label publication reads and borrowed bytes | metadata and publication tests |
| `CTR-ITER-001` | bounded dense scans and first-match searches | `tests/counters_reader_iteration.rs` |
| `CTR-LIFE-001` | borrowed lifetime and private unsafe module | API/source inspection and tests |
| `CTR-ALLOC-001` | allocation-free read and scan implementation | `tests/counters_reader_allocation.rs` |
| `CTR-PORT-001` | native-atomic guard and existing CI matrix | local MSRV plus post-merge CI |

## Capability gate

`COUNTER-READER` can close when every reader requirement is implemented and
validated on the claimed surfaces. `COUNTER-FAMILY` remains partial until
`CountersManager`, `AtomicCounter`, and applicable position/status types are
implemented and verified.

