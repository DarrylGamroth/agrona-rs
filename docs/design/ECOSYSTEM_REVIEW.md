# Rust ecosystem review

> Maintainer design record. For package usage, see the
> [User Guide](../USER_GUIDE.md).

## Status

This is the recorded G2 ecosystem input for the
[initial delivery plan](PORTING_PLAN.md). It was reviewed on 2026-07-27 against
Agrona commit `d4a47c67258f85b39910c4999da346ead655b736`.

The delivery plan, rather than this review, selects Clocks and the Agent
protocol, runner, invoker, static composite, and idle strategies. Dynamic
composition is not selected. Versions are a review snapshot and must be
checked again before adding a dependency.

## Why Agrona's zero-GC design still matters

Agrona was designed for predictable low-latency operation on a managed Java
runtime. Avoiding allocation on a steady-state hot path avoids garbage
collection pressure and is directly relevant to Julia.

Rust has no tracing garbage collector, but that does not make the underlying
design constraints irrelevant. Preallocated bounded storage, no steady-state
allocator traffic, cache-conscious layouts, explicit overload behavior,
single-writer ownership, and bounded work per duty cycle are still useful for
predictable latency. An existing Rust crate is comparable only when it
satisfies the required operational contract, not merely because it has a
similarly named data structure.

For every candidate, compare:

- producer and consumer cardinality;
- fixed capacity, preallocation, and growth behavior;
- allocator use during steady-state operations;
- progress guarantees and blocking behavior;
- full, empty, lag, loss, and recovery semantics;
- ordering and publication guarantees;
- synchronous polling versus asynchronous wakeups;
- item, byte, or typed-record storage;
- work bounds for each poll or duty cycle;
- thread placement and operating-system affinity;
- clock rollback and time-source behavior where applicable; and
- supported targets, MSRV, `std`/`no_std`, safety boundaries, and maintenance.

Claims such as “lock-free,” “zero-copy,” and “bounded” are screening inputs,
not proof of comparable behavior. A selected dependency still requires source
review, focused tests, and representative benchmarks.

## Disposition vocabulary

- **Adopt** — evaluate the existing crate as the implementation.
- **Wrap** — use an existing implementation behind a small domain-specific
  contract when application-level substitution or testing requires it.
- **Port candidate** — Agrona supplies relevant semantics for which no
  sufficiently comparable maintained Rust implementation was found.
- **Selected port** — the delivery plan has approved an Agrona-derived
  implementation after the other selection gates were considered.
- **Omit** — Rust or mature crates already cover the capability and an
  Agrona-shaped API would add no demonstrated value.
- **Defer** — keep outside the current selection round.

“Port candidate” is not “selected.” It must still pass G1, G3, G4, and G5.

## Working comparison

| Agrona area | Comparable Rust choices | Semantic assessment | Working G2 disposition |
|---|---|---|---|
| Agent protocol and runner | Rust threads; actor frameworks such as [`steady_state`](https://docs.rs/steady_state/) | No maintained crate found with the same small duty-cycle protocol, bounded `do_work` loop, lifecycle, error, idle, composite, and invoker semantics. General actor runtimes impose a different execution model. | **Selected port** by the delivery plan. |
| Thread placement | [`core_affinity`](https://docs.rs/core_affinity/) | Supplies operating-system CPU affinity. It complements an Agent runner; selecting a Julia or Rust scheduler thread is not equivalent to pinning it to a CPU. | **Adopt** as an optional substrate if affinity is selected. |
| Idle strategies | [`idle`](https://docs.rs/idle/) and `std::thread` | `idle` provides no-op, spin, and sleep, but not the full yield/backoff/reset/controllable family. Any busy strategy also needs a deployment contract covering reserved cores and power. | **Selected port** of all Agrona strategies. |
| Epoch and monotonic clocks | `std::time`; [`quanta`](https://docs.rs/quanta/); [`coarsetime`](https://docs.rs/coarsetime/) | These can supply time sources, but none establishes the complete Agrona contract: distinct epoch millisecond, epoch microsecond, epoch nanosecond, and arbitrary-origin monotonic nanosecond domains; manually driven cached clocks; or offset-clock sampling behavior. | **Selected port** for behavioral compatibility. **Adopt/evaluate** an existing source only as an internal backend when it preserves that contract. |
| Snowflake IDs | [`snowflake_me`](https://docs.rs/snowflake_me/); [`snowdon`](https://docs.rs/snowdon/) | Existing crates provide configurable layouts and concurrent generation. `snowflake_me` also exposes clock-regression policies and batch generation. Exact Agrona bit layout and rollback behavior still need differential checks if compatibility matters. | **Adopt** a reviewed crate by default; port only for an unmet exact compatibility contract. |
| Direct and mutable buffers | slices; [`bytes`](https://docs.rs/bytes/); [`zerocopy`](https://docs.rs/zerocopy/); [`bytemuck`](https://docs.rs/bytemuck/) | Rust already has borrowed byte views, owned contiguous buffers, and checked typed views. These do not automatically reproduce Agrona offset methods or binary protocol contracts. | **Omit/compose** by default; port only as part of selected binary interoperability. |
| Atomic buffers | `std::sync::atomic`; [`portable-atomic`](https://docs.rs/portable-atomic/); [`atomic`](https://docs.rs/atomic/) | Existing atomics cover typed atomic values. An Agrona atomic byte buffer additionally defines alignment, byte layout, offset, and publication semantics. | **Adopt** typed atomics; retain an atomic-buffer **port candidate** only for an exact selected record protocol. |
| Bounded SPSC queue | [`rtrb`](https://docs.rs/rtrb/) | Fixed-capacity split producer/consumer ownership with no allocation after construction is close to the relevant latency contract. | **Adopt/evaluate** before porting. |
| Bounded MPMC/MPSC queues | [`crossbeam_queue::ArrayQueue`](https://docs.rs/crossbeam-queue/latest/crossbeam_queue/struct.ArrayQueue.html); [`thingbuf`](https://docs.rs/thingbuf/) | Mature bounded alternatives exist, but cardinality, slot reuse, cache layout, and overload behavior differ. | **Adopt/evaluate** the structure matching the selected cardinality; port only after a measured semantic gap. |
| One-to-one message ring | [`bbqueue`](https://docs.rs/bbqueue/) | Provides bounded SPSC byte storage and framed grants. It is close, but does not by itself establish Agrona message type, padding, correlation, or record layout behavior. | **Adopt/evaluate** for application-local frames; **port candidate** for exact Agrona record semantics. |
| Many-to-one message ring | General bounded queues | No mature direct equivalent was found for Agrona's bounded variable-length record and publication contract. Item queues are not substitutes. | **Port candidate** if a G1 use case needs those exact semantics. |
| Broadcast buffer | [`tokio::sync::broadcast`](https://docs.rs/tokio/latest/tokio/sync/broadcast/); [`bus`](https://docs.rs/bus/) | Tokio provides bounded overwrite and lag notification in an async, cloned-value model. `bus` provides bounded broadcast with different slow-receiver behavior. Neither is an exact synchronous byte-record replacement. | **Adopt** for compatible async applications; **port candidate** for synchronous polling, independent cursors, and exact lapping semantics. |
| Expandable ring buffer | `Vec`, `VecDeque`, and `bytes` | Standard growable storage covers general use. Agrona's value would have to come from its record API and measured reuse behavior. | **Omit/compose** unless a concrete record-oriented use case demonstrates a gap. |
| Deadline timer wheel | [`nexus-timer`](https://docs.rs/nexus-timer/); [`hierarchical_hash_wheel_timer`](https://docs.rs/hierarchical_hash_wheel_timer/) | Existing crates offer manually polled wheels, cancellation, and bounded polling. Their deadline ordering, capacity, overflow, and allocation contracts must be checked against the use case. | **Adopt/evaluate `nexus-timer` first**; do not port by default. |
| Distinct error log | [`tracing`](https://docs.rs/tracing/); [`tracing_throttle`](https://docs.rs/tracing-throttle/) | Existing observability can deduplicate or throttle recurring events. It does not reproduce Agrona's bounded first/last/count record format and reader contract. | **Adopt** observability tooling unless that exact in-memory record contract is required. |
| Common ID abstraction | Concrete ID crates | A common trait adds indirection and policy without value when only one generator is selected. | **Omit** until at least two selected implementations require substitution. |
| Primitive collections and caches | standard collections; [`hashbrown`](https://docs.rs/hashbrown/); [`lru`](https://docs.rs/lru/); [`quick_cache`](https://docs.rs/quick_cache/) | Rust monomorphization removes Java primitive-boxing pressure. Specialized eviction or memory-layout behavior still needs a concrete benchmark to justify a port. | **Omit** by default; evaluate an individual cache only for a demonstrated latency contract. |
| Checksums | [`crc32fast`](https://docs.rs/crc32fast/); [`crc32c`](https://docs.rs/crc32c/) | Mature implementations already include architecture-specific acceleration. | **Adopt directly** where needed; do not reimplement or facade-export without an API reason. |
| ASCII and numeric encoding | [`lexical-core`](https://docs.rs/lexical-core/); [`btoi`](https://docs.rs/btoi/); [`itoa`](https://docs.rs/itoa/) | Existing crates parse and format directly against byte slices. | **Adopt**; port only exact edge behavior required by a selected wire protocol. |
| I/O and NIO helpers | `std::io`; [`mio`](https://docs.rs/mio/) | Much of Agrona's surface adapts Java I/O and selector APIs rather than defining a portable protocol. | **Omit** unless an individual non-Java semantic gap is identified. |
| Code generation and Java agent | Rust generics, macros, and compiler layout tools | Primitive specialization and Java instrumentation solve Java-specific problems. | **Omit**. |
| Counters, positions, mark files, and mapped IPC | [`memmap2`](https://docs.rs/memmap2/) and OS primitives | Existing crates provide memory-mapping substrate, not Agrona's cross-process layouts, lifecycle, liveness, and publication protocols. | **Defer** with the rest of shared memory. |

## Strongest initial conclusions

### Agent is selected as a complete port

No reviewed crate is a drop-in equivalent to Agrona's deliberately small
duty-cycle protocol. The initial delivery therefore includes the Agent
protocol, runner, invoker, static composite, lifecycle and error behavior, and
all Agrona idle strategies. `DynamicCompositeAgent` is omitted because the
initial use case does not require cross-thread reconfiguration. Actual CPU
affinity remains an explicit optional facility rather than a claim made from
thread placement alone.

### Clocks are selected from the Agrona Java contract

Agrona supplies a coherent clock contract needed by Rust actor and duty-cycle
applications:

- separate provider contracts for epoch milliseconds, epoch microseconds,
  epoch nanoseconds, and arbitrary-origin monotonic nanoseconds;
- allocation-free system reads;
- manually driven cached epoch and monotonic clocks with one updating owner,
  acquire reads, and release publication;
- offset epoch nanoseconds derived by sampling an epoch source between two
  monotonic reads, choosing the narrowest measurement window, and reporting
  whether the configured threshold was met; and
- resampling after the configured interval or when monotonic time moves
  backwards.

The G3 choice is **behavioral compatibility** with Agrona Java, not binary
layout compatibility. Rust traits and ownership remain idiomatic, but they
must not collapse epoch and monotonic domains or move cached-clock refresh
policy out of the owning actor. Existing clock crates can still be evaluated
as system-source backends.

Clock G5 includes tests derived from the recorded Agrona revision,
deterministic injected-source tests, concurrency tests for cached publication,
boundary tests for offset resampling, and steady-state allocation and latency
evidence for every read path. `Clocks.jl` is an example only and is not a test
oracle.

### Snowflake IDs should begin as a dependency evaluation

Rust already has credible Snowflake implementations. A local abstraction may
still be useful for dependency injection or deterministic tests, but it should
remain small. A port becomes preferable only when a written compatibility
requirement identifies layout or clock-regression behavior that the reviewed
crates cannot supply.

### Queue family names are not enough

The ecosystem has strong bounded queues, but SPSC, MPSC, and MPMC structures
are not interchangeable. Item queues are also not substitutes for
variable-length byte-record rings. Selection must compare the precise
cardinality, storage, allocation, backpressure, and publication contract.

### Async broadcast is not automatically an Agrona broadcast buffer

Tokio's lag reporting is a useful semantic analogue, but its task wakeups,
value cloning, and async receive model may be inappropriate for an Agent
duty-cycle. It should be adopted when that model fits the application, not
used to close the question for synchronous low-latency polling.

## Dependency acceptance checklist

Before changing an **Adopt** or **Wrap** recommendation into a selection:

1. Pin the required semantics and workload from G1.
2. Review the dependency's public contract and relevant implementation,
   including `unsafe` code and atomic ordering.
3. Check license, MSRV, supported targets, feature defaults, transitive
   dependencies, and `std`/`no_std` implications.
4. Confirm fixed-capacity and steady-state allocation behavior with focused
   tests or allocator instrumentation.
5. Exercise full, empty, lag, cancellation, rollback, wraparound, and shutdown
   paths as applicable.
6. Benchmark the intended cardinality and offered-load pattern, including
   overload and tail latency.
7. Record the dependency version and a fallback plan in the component decision
   record.

Download counts and recent releases can help prioritize review, but they are
not correctness, soundness, or latency evidence.
