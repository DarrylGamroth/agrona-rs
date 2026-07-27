# agrona-rs

[![CI](https://github.com/DarrylGamroth/agrona-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/DarrylGamroth/agrona-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/DarrylGamroth/agrona-rs/graph/badge.svg)](https://codecov.io/gh/DarrylGamroth/agrona-rs)

Project skeleton for an unofficial, idiomatic Rust port of selected
[Agrona](https://github.com/aeron-io/agrona) low-latency building blocks.

> [!WARNING]
> The Clock family is implemented. The complete Agent family is selected as
> the next component but has not been implemented yet.

The Clock implementation includes distinct epoch and monotonic provider
traits, system clocks, single-writer cached clocks, injectable sources, and an
offset epoch nanosecond clock.

## Clocks

```rust
use agrona::clock::{
    CachedEpochClock, EpochClock, EpochNanoClock, OffsetEpochNanoClock,
    SystemEpochClock,
};

let epoch_ms = SystemEpochClock.time();

let mut cached = CachedEpochClock::with_initial_time(epoch_ms);
let reader = cached.reader();
cached.advance(1);
assert_eq!(epoch_ms + 1, reader.time());

let offset = OffsetEpochNanoClock::new()?;
let epoch_ns = offset.nano_time();
# Ok::<(), agrona::clock::OffsetEpochNanoClockError>(())
```

Cached writers are not cloneable; cloned reader handles observe release
published updates with acquire ordering. Normal offset-clock reads are
lock-free and allocation-free.

## Planning

See [the initial delivery plan](docs/PORTING_PLAN.md) for:

- the selected Clock and Agent component families;
- their Rust ownership and compatibility decisions;
- dependency-ordered implementation and validation phases;
- explicitly deferred shared-memory facilities; and
- the API decisions that must be reviewed before source implementation.

The normative Clock contract and current evidence are in
[`CLOCK_SPEC.md`](docs/CLOCK_SPEC.md) and
[`CLOCK_EVIDENCE.md`](docs/CLOCK_EVIDENCE.md).

See the [Rust ecosystem review](docs/ECOSYSTEM_REVIEW.md) for the current
feature-by-feature adopt, wrap, port-candidate, omit, or defer recommendations.
Those recommendations compare operational semantics—including steady-state
allocation and latency bounds—not just similarly named APIs.

Agrona Java is the normative behavioral reference. Aeron C is an
implementation reference for native Agent ownership, thread lifecycle, and
idle primitives. The sibling `Agent.jl`, `Clocks.jl`, and `SnowflakeId.jl`
packages are independent examples only; they are not acceptance oracles for
the Rust port.

## Minimum supported Rust version

The skeleton provisionally uses Rust 1.85 and Rust 2024 edition. The supported
compiler policy will be confirmed before the first release.

## License

Copyright 2026 Rubus Technologies Inc.

Licensed under the [Apache License, Version 2.0](LICENSE). Portions are adapted
from Agrona only when identified in future source files. See [NOTICE](NOTICE)
and [UPSTREAM.md](UPSTREAM.md) for planned attribution policy.
