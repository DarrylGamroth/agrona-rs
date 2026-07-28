# agrona-rs user guide

`agrona-rs` is an unofficial, idiomatic Rust port of selected
[Agrona](https://github.com/aeron-io/agrona) low-latency components. The
current crate provides clocks, a synchronous Agent framework, static Agent
composition, and all seven Agrona idle strategies.

This is not an async runtime or a general actor framework. An Agent is a
single-owner duty cycle: one thread repeatedly asks it to do a bounded amount
of work.

## Installation

Releases are distributed through GitHub rather than crates.io. Pin the release
tag so that builds are reproducible:

```toml
[dependencies]
agrona = { git = "https://github.com/DarrylGamroth/agrona-rs", tag = "v0.1.1" }
```

Rust 1.85 or newer is required. The crate requires native 64-bit atomics.

## Clocks

Import the trait for the time domain you need, then call the corresponding
method on a provider:

```rust
use agrona::clock::{
    EpochClock, EpochMicroClock, EpochNanoClock, NanoClock, SystemEpochClock,
    SystemEpochMicroClock, SystemEpochNanoClock, SystemNanoClock,
};

let epoch_ms = SystemEpochClock.time();
let epoch_us = SystemEpochMicroClock.micro_time();
let epoch_ns = SystemEpochNanoClock.nano_time();

let start_ns = SystemNanoClock.nano_time();
// Do bounded work here.
let elapsed_ns = SystemNanoClock.nano_time().wrapping_sub(start_ns);

assert!(epoch_ms > 0);
assert!(epoch_us > epoch_ms);
assert!(epoch_ns > epoch_us);
assert!(elapsed_ns >= 0);
```

Choose clocks by meaning, not only by resolution:

| Need | Trait and default provider | Notes |
|---|---|---|
| Wall-clock milliseconds | `EpochClock`, `SystemEpochClock` | Milliseconds since the Unix epoch |
| Wall-clock microseconds | `EpochMicroClock`, `SystemEpochMicroClock` | Resolution depends on the operating system |
| Wall-clock nanoseconds | `EpochNanoClock`, `SystemEpochNanoClock` | Resolution may be coarser than one nanosecond |
| Elapsed time | `NanoClock`, `SystemNanoClock` | Monotonic, process-local, arbitrary origin |
| Cheap repeated reads | `CachedEpochClock` or `CachedNanoClock` | One writer publishes to cloneable readers |
| Epoch nanoseconds derived from monotonic time | `OffsetEpochNanoClock` | Periodically resamples its epoch offset |

Do not subtract epoch timestamps to measure short elapsed intervals when a
monotonic clock is available. `NanoClock` values can wrap, so use wrapping
subtraction as shown above.

### Cached clocks

A cached clock has exactly one mutable writer. Read-only handles are cloneable
and may be sent to other threads:

```rust
use agrona::clock::{CachedEpochClock, EpochClock};

let mut writer = CachedEpochClock::with_initial_time(1_000);
let reader = writer.reader();
let another_reader = reader.clone();

writer.update(1_250);
assert_eq!(1_250, reader.time());
assert_eq!(1_250, another_reader.time());

assert_eq!(1_260, writer.advance(10));
assert_eq!(1_260, reader.time());
```

`update` and `advance` are single-writer operations. They do not turn a cached
clock into a multi-writer atomic counter.

### Offset epoch nanoseconds

`SystemEpochNanoClock` reads the system wall clock directly.
`OffsetEpochNanoClock` instead samples epoch milliseconds and advances that
sample with a monotonic nanosecond clock:

```rust
use agrona::clock::{EpochNanoClock, OffsetEpochNanoClock};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = OffsetEpochNanoClock::new()?;
let epoch_ns = clock.nano_time();

assert!(epoch_ns > 0);
let _sample_met_threshold = clock.is_within_threshold();
# Ok(())
# }
```

Use `OffsetEpochNanoClockConfig` and `OffsetEpochNanoClock::with_sources` when
you need custom sampling thresholds, resampling intervals, or deterministic
clock sources for a test.

## Agents

Implement `Agent` for a type that owns the state touched by its duty cycle:

```rust
use agrona::agent::{Agent, AgentResult, AgentTermination};

struct CounterAgent {
    remaining: u64,
}

impl Agent for CounterAgent {
    fn role_name(&self) -> &str {
        "counter"
    }

    fn on_start(&mut self) -> Result<(), agrona::agent::BoxError> {
        Ok(())
    }

    fn do_work(&mut self) -> AgentResult {
        if self.remaining == 0 {
            return Err(AgentTermination::expected().into());
        }

        self.remaining -= 1;
        Ok(1)
    }

    fn on_close(&mut self) -> Result<(), agrona::agent::BoxError> {
        Ok(())
    }
}
```

`do_work` returns a signed work count. Return a positive value when useful
work was completed and zero when no work was available. The owner passes this
count to its idle strategy, which resets after positive work and idles after
zero or negative work.

Keep each duty cycle bounded and non-blocking. Natural batching is useful, but
an unbounded batch delays shutdown and prevents other Agents in a composite
from making progress.

### Dedicated thread with `AgentRunner`

`AgentRunner` transfers the Agent to a named operating-system thread. The
Agent's `role_name` becomes the thread name:

```rust
use std::error::Error;

use agrona::agent::{
    Agent, AgentResult, AgentRunner, AgentTermination, BackoffIdleStrategy,
};

#[derive(Debug)]
struct CounterAgent {
    remaining: u64,
}

impl Agent for CounterAgent {
    fn role_name(&self) -> &str {
        "counter"
    }

    fn do_work(&mut self) -> AgentResult {
        if self.remaining == 0 {
            return Err(AgentTermination::expected().into());
        }
        self.remaining -= 1;
        Ok(1)
    }
}

fn report_error(error: &(dyn Error + Send + Sync + 'static)) {
    eprintln!("counter Agent: {error}");
}

# fn main() -> Result<(), Box<dyn Error>> {
let runner = AgentRunner::new(
    CounterAgent { remaining: 3 },
    BackoffIdleStrategy::default(),
    report_error,
    None,
);

let handle = runner.start()?;
let agent = handle.join()?;
assert_eq!(0, agent.remaining);
# Ok(())
# }
```

Call `join` when the Agent terminates itself. To stop it from another thread,
call `close`, which requests stop, waits for `on_close`, joins the thread, and
returns the owned Agent:

```rust
use std::error::Error;

use agrona::agent::{Agent, AgentResult, AgentRunner, BackoffIdleStrategy};

struct Service;

impl Agent for Service {
    fn role_name(&self) -> &str {
        "service"
    }

    fn do_work(&mut self) -> AgentResult {
        Ok(0)
    }
}

fn ignore_error(_: &(dyn Error + Send + Sync + 'static)) {}

# fn main() -> Result<(), Box<dyn Error>> {
let handle = AgentRunner::new(
    Service,
    BackoffIdleStrategy::default(),
    ignore_error,
    None,
)
.start()?;

let _service = handle.close()?;
# Ok(())
# }
```

Shutdown is cooperative. A blocking `do_work` call must have an
application-owned wakeup or cancellation mechanism. `request_stop` unparks a
parked worker, but it cannot cancel arbitrary blocking I/O. Use
`close_with_retry` to receive periodic stall callbacks while waiting.

Do not simply drop a live `AgentRunnerHandle`: like `std::thread::JoinHandle`,
that detaches the worker without requesting stop and loses deterministic
cleanup.

### Worker initialization

`start_with_thread_initializer` runs a caller-supplied initializer once on the
worker thread before `Agent::on_start`. Use it for thread-local deployment
configuration such as CPU affinity, scheduling policy, or thread-local
runtime setup:

```rust
use std::error::Error;

use agrona::agent::{
    Agent, AgentResult, AgentRunner, AgentTermination, BoxError,
    NoOpIdleStrategy,
};

struct Service;

impl Agent for Service {
    fn role_name(&self) -> &str {
        "pinned-service"
    }

    fn do_work(&mut self) -> AgentResult {
        Err(AgentTermination::expected().into())
    }
}

fn ignore_error(_: &(dyn Error + Send + Sync + 'static)) {}

# fn main() -> Result<(), Box<dyn Error>> {
let runner = AgentRunner::new(
    Service,
    NoOpIdleStrategy,
    ignore_error,
    None,
);

let handle = runner.start_with_thread_initializer(|| -> Result<(), BoxError> {
    // Apply application- or platform-specific worker configuration here.
    Ok(())
})?;
let _service = handle.join()?;
# Ok(())
# }
```

An initializer error is reported through the runner's error handler. It
prevents `Agent::on_start` and `do_work`, then the runner still attempts
`Agent::on_close`. An initializer panic is fatal and is returned through
`join` after cleanup is attempted. Initialization adds no callback, branch,
or allocation to the steady-state duty-cycle loop.

This is the Rust equivalent of Agrona Java's `ThreadFactory` customization
point. The crate does not select an affinity library or impose operating
system policy. Applications can call their chosen platform integration from
the initializer. Every sub-agent in a `CompositeAgent` runs on the same
owning thread and therefore shares its thread configuration; use separate
runners when Agents need different placement.

### Caller-driven execution with `AgentInvoker`

Use `AgentInvoker` when an existing thread or event loop should own the duty
cycle:

```rust
use std::error::Error;

use agrona::agent::{Agent, AgentInvoker, AgentResult, AgentTermination};

struct PollOnce(bool);

impl Agent for PollOnce {
    fn role_name(&self) -> &str {
        "poll-once"
    }

    fn do_work(&mut self) -> AgentResult {
        if self.0 {
            Err(AgentTermination::expected().into())
        } else {
            self.0 = true;
            Ok(1)
        }
    }
}

fn ignore_error(_: &(dyn Error + Send + Sync + 'static)) {}

let mut invoker = AgentInvoker::new(PollOnce(false), ignore_error, None);
invoker.start();

assert_eq!(1, invoker.invoke());
assert_eq!(0, invoker.invoke());
assert!(invoker.is_closed());
```

An invoker is intentionally not thread-safe. The calling code owns and
serializes `start`, `invoke`, and `close`.

### Static composition

`CompositeAgent` gives one owner a fixed group of heterogeneous Agents:

```rust
use agrona::agent::{Agent, AgentResult, CompositeAgent};

struct Poller(&'static str);

impl Agent for Poller {
    fn role_name(&self) -> &str {
        self.0
    }

    fn do_work(&mut self) -> AgentResult {
        Ok(0)
    }
}

let composite = CompositeAgent::new(vec![
    Box::new(Poller("network")),
    Box::new(Poller("timers")),
])
.expect("a composite requires at least one Agent");

assert_eq!("[network,timers]", composite.role_name());
```

Sub-agents run sequentially on the owning thread. Lifecycle failures from all
sub-agents are collected in encounter order. `DynamicCompositeAgent` is not
implemented; build a new static composite when membership changes.

## Choosing an idle strategy

All strategies implement `IdleStrategy` and can be used with `AgentRunner`:

| Strategy | Alias | Idle behavior | Typical tradeoff |
|---|---|---|---|
| `NoOpIdleStrategy` | `noop` | Does nothing | Caller supplies all pacing |
| `BusySpinIdleStrategy` | `spin` | CPU spin hint | Lowest response time, consumes a core |
| `YieldingIdleStrategy` | `yield` | Yields the thread | Low delay with scheduler involvement |
| `BackoffIdleStrategy` | `backoff` | Spins, yields, then parks | General-purpose adaptive default |
| `SleepingIdleStrategy` | `sleep-ns` | Parks for a `Duration` | Lower CPU use; OS wakeup granularity applies |
| `SleepingMillisIdleStrategy` | `sleep-ms` | Sleeps for a `Duration` | Coarser pacing and lower CPU use |
| `ControllableIdleStrategy` | `controllable` | Atomically selected mode | Runtime control from another owner |

`BackoffIdleStrategy::default()` uses Agrona's standard thresholds. Choose
busy-spin only when dedicating a CPU is an intentional deployment decision.
Actual sleep and park durations are lower bounds and depend on the operating
system scheduler.

A controllable strategy returns a strategy/control pair. Keep the strategy
with the runner and send clones of the control handle to configuration code:

```rust
use agrona::agent::{
    ControllableIdleStrategy, ControllableIdleStrategyMode, IdleStrategy,
};

let (mut strategy, control) = ControllableIdleStrategy::new();
assert_eq!("controllable", strategy.alias());

control.set(ControllableIdleStrategyMode::BusySpin);
strategy.idle(0);

control.set(ControllableIdleStrategyMode::Yield);
assert_eq!(ControllableIdleStrategyMode::Yield as i32, control.raw());
```

Mode publication uses release/acquire atomic ordering and does not require a
mutex.

## Errors and termination

Agent failures have distinct meanings:

- Return `Ok(work_count)` after an ordinary duty cycle.
- Return `AgentError::failed(error)` for a recoverable work failure. The error
  handler is called, the optional `AgentErrorCounter` increments, and
  execution continues.
- Return `AgentTermination::expected().into()` for a normal, quiet stop.
- Return `AgentTermination::unexpected().with_message(...).into()` when
  termination should also be reported to the error handler.
- Return an error from `on_start` or `on_close` for a lifecycle failure. These
  are reported but are not included in `AgentErrorCounter`.

The error handler must not panic. A panic from Agent code remains fatal:
`AgentRunner` attempts `on_close`, and `join` or `close` returns an
`AgentRunnerJoinError` containing the Agent and panic payloads.

`AgentRunnerStartError::into_parts` returns both the operating-system error
and the still-unstarted runner, so ownership is not lost if thread creation
fails.

## Current scope

The crate does not currently implement shared-memory buffers, counters,
queues, ring buffers, or control structures. It also does not provide
`DynamicCompositeAgent`, CPU affinity, automatic CPU reservation,
scheduling-priority or NUMA policy, async integration, or `no_std` support.

For exact public types and methods, build and open the API documentation:

```text
cargo doc --open
```
