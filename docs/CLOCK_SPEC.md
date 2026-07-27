# agrona-rs Clock specification

Status: active

Baseline: Agrona Java commit
`d4a47c67258f85b39910c4999da346ead655b736`

Applicable profile: the initial `std` profile on Linux, macOS, and Windows
targets with native 64-bit atomics.

## Purpose and authority

This document is the normative Rust contract for the Clock increment selected
by `DEC-CLOCK-001` in the [initial delivery plan](PORTING_PLAN.md). Agrona Java
is the behavioral reference. The delivery plan owns roadmap priority and
component boundaries; this document owns the Clock API and behavior.

`Clocks.jl` is an example only. It is not a normative dependency or test
oracle.

## Normative language

Uppercase requirement terms use the meanings defined by BCP 14 (RFC 2119 and
RFC 8174). Lowercase forms are ordinary prose.

## Terminology and units

- **Epoch milliseconds**, **epoch microseconds**, and **epoch nanoseconds** are
  signed `i64` counts since 1 January 1970 UTC.
- **Monotonic nanoseconds** are signed `i64` ticks from an unspecified,
  process-local origin and are suitable only for elapsed-time measurement.
- **Normal read path** means a read that does not trigger offset resampling.
- **Steady state** means operation after all objects and process-local clock
  state have been initialized.

## Requirements

### CLK-DOM-001 — Distinct time domains

The `clock` module MUST expose object-safe `EpochClock::time`,
`EpochMicroClock::micro_time`, `EpochNanoClock::nano_time`, and
`NanoClock::nano_time` provider traits returning the signed units defined
above, without a common provider trait that permits epoch and monotonic
nanoseconds to be substituted accidentally.

Verification intent (informative): compile-time API tests and documentation
examples establish the four independent contracts.

### CLK-SYS-001 — System epoch clocks

`SystemEpochClock`, `SystemEpochMicroClock`, and `SystemEpochNanoClock` MUST be
zero-sized providers whose reads derive the corresponding signed epoch unit
from `std::time::SystemTime`, retain wrapping `i64` arithmetic at numeric
overflow, and document that precision may be coarser than the return unit.

Verification intent (informative): size, type, current-time range, and numeric
conversion tests.

### CLK-MONO-001 — System monotonic clock

`SystemNanoClock` MUST be a zero-sized provider that returns nondecreasing
process-local monotonic nanosecond ticks during ordinary execution and retains
wrapping `i64` arithmetic when the representable range is exceeded.

Verification intent (informative): size, repeated-ordering, and elapsed-time
tests.

### CLK-CACHE-001 — Cached clock ownership

`CachedEpochClock` and `CachedNanoClock` MUST each create exactly one
non-cloneable writer that also implements its provider trait and any number of
cloneable read-only handles named `CachedEpochClockReader` and
`CachedNanoClockReader`, with an initial value of zero unless an explicit
initial value is supplied.

Verification intent (informative): public API, initial-value, and compile-fail
ownership documentation tests.

### CLK-CACHE-002 — Cached clock publication

Cached clock `update` and wrapping `advance` operations MUST require mutable
access to the unique writer, publish with release ordering, and be observed by
provider reads with acquire ordering.

Verification intent (informative): update, wraparound, multi-reader stress,
and source inspection.

### CLK-OFFSET-001 — Offset configuration and construction

`OffsetEpochNanoClockConfig` and `OffsetEpochNanoClockError` MUST provide
fallible construction that rejects a zero retry count, a nanosecond threshold
outside `0..=i64::MAX`, or a nanosecond resample interval outside
`1..=i64::MAX`, while the default configuration uses 100 retries, a 250 ns
threshold, and a one-hour interval.

Verification intent (informative): default and every invalid boundary are
tested.

### CLK-OFFSET-002 — Offset sampling

`OffsetEpochNanoClock<E, N>` with `E: EpochClock` and `N: NanoClock` MUST
sample monotonic time, epoch milliseconds, and monotonic time in that order;
use the midpoint of a non-negative window; publish the first window narrower
than the configured threshold or otherwise the narrowest valid window after
the bounded retry count; and expose the result through `sample` and
`is_within_threshold`.

Verification intent (informative): deterministic scripted sources exercise
first-match, narrowest fallback, strict threshold, retry bound, midpoint,
conversion, and no-valid-window behavior.

### CLK-OFFSET-003 — Coherent offset reads and resampling

Offset clock reads MUST observe one coherent published sample without taking
the sampling mutex on the normal read path, derive epoch nanoseconds with
wrapping `i64` arithmetic, and serialize resampling after interval expiry or
backward monotonic movement.

Verification intent (informative): publication stress, interval, backward
movement, explicit concurrent sampling, and source inspection.

### CLK-OFFSET-004 — Automatic resampling failure

When an automatic resampling attempt cannot obtain a valid measurement,
`OffsetEpochNanoClock` MUST retain the last coherent sample and return a value
derived from that sample, while explicit `sample` reports
`OffsetEpochNanoClockError::NoValidSample`.

Verification intent (informative): a scripted source forces every automatic
sample window backward and verifies retained-sample behavior.

### CLK-ALLOC-001 — Steady-state allocation

System reads, cached reads, cached updates, cached advances, offset normal
reads, and threshold-status reads MUST perform zero heap allocations in steady
state.

Verification intent (informative): a dedicated integration-test binary uses a
counting global allocator after warmup.

### CLK-PORT-001 — Supported implementation profile

The Clock implementation MUST use safe Rust in the library, require `std` and
native 64-bit atomics, compile at the provisional Rust 1.85 MSRV, and keep its
behavioral test suite enabled on Linux, macOS, and Windows.

Verification intent (informative): source inspection plus stable, MSRV, and
three-OS CI.

## Claim limits

Conformance to this specification does not claim binary compatibility with
Agrona Java, identical clock resolution across operating systems, a `no_std`
profile, or performance equivalence with Java or either Julia example.
