# agrona-rs Agent specification

> Maintainer specification. For package usage, see the
> [User Guide](../../USER_GUIDE.md).

Status: active

Baseline: Agrona Java commit
`d4a47c67258f85b39910c4999da346ead655b736`

Applicable profile: `std` on Linux, macOS, and Windows with native
pointer-width and 64-bit atomics. `DynamicCompositeAgent` is excluded.

## Purpose and authority

This document is the normative Rust contract for the Agent increment selected
by `DEC-AGENT-001` in the [delivery plan](../PORTING_PLAN.md). Agrona Java is the
behavioral authority. Aeron C is a native ownership and atomic-ordering
reference. `Agent.jl` is an example only.

## Normative language

Uppercase requirement terms use the meanings defined by BCP 14 (RFC 2119 and
RFC 8174). Lowercase forms are ordinary prose.

## Requirements

### AGT-CORE-001 — Agent contract

The `agent` module MUST expose an object-safe, mutable, `Send + 'static`
`Agent` trait with a borrowed role name, default successful `on_start` and
`on_close`, a fallible `do_work` returning a signed `i32` work count, and
single-owner serialization of lifecycle and duty-cycle calls.

Verification intent (informative): API, default lifecycle, object-safety, and
single-owner tests.

### AGT-ERR-001 — Typed failure and termination

Recoverable duty-cycle failure and Agent termination MUST be distinct typed
`AgentError` variants that retain expected/unexpected termination state, keep
expected termination quiet, report unexpected termination without incrementing
the ordinary work-error counter, and never convert panics into recoverable
Agent errors.

Verification intent (informative): typed API and handling-matrix tests.

### AGT-ERR-002 — Error reporting and counting

The owner MUST report ordinary `do_work` failures, increment a configured
process-local wrapping `i64` counter only while still running, permit the next
duty cycle, and report startup and cleanup failures without incrementing that
counter. An error-handler panic is fatal.

Verification intent (informative): origin-by-origin handler and counter tests.

### AGT-INV-001 — Caller-owned invocation

`AgentInvoker` MUST be caller-owned and internally unlocked, attempt startup
at most once, return zero when not running or after handling a failure,
continue after ordinary work failure, close after termination or startup
failure, and attempt cleanup at most once.

Verification intent (informative): state, lifecycle, failure, termination,
idempotence, and panic tests.

### AGT-RUN-001 — Dedicated-thread lifecycle

Starting `AgentRunner` MUST transfer one Agent and one idle strategy to one
named OS thread that calls startup once, repeatedly calls `idle(do_work())`
while running, attempts cleanup once on every non-abort exit, and contains no
runner-introduced mutex, channel operation, or heap allocation in its
steady-state loop.

Verification intent (informative): lifecycle ordering, thread identity,
failure, panic, and allocation tests.

### AGT-RUN-002 — Cooperative stop and ownership

Runner control MUST release-publish and acquire-observe a non-blocking stop,
unpark the worker, preserve the unstarted runner on OS spawn failure, make
join wait for cleanup and return the owned Agent or a structured fatal result
retaining the Agent and panic payload, and never claim forced cancellation of
blocking Agent code.

Verification intent (informative): stop races, close-before-start,
spawn-failure injection, join, panic, and blocked-Agent diagnostics.

### AGT-RUN-003 — Worker initialization

A caller-supplied runner initializer MUST execute once on the named worker
thread before `Agent::on_start`. A recoverable initializer error MUST be
reported without incrementing the work-error counter, MUST prevent Agent
startup and duty cycles, and MUST be followed by one cleanup attempt. An
initializer panic MUST remain fatal and MUST follow the runner's structured
panic and cleanup behavior. Initialization MUST add no operation to the
steady-state duty-cycle loop.

Verification intent (informative): worker identity and lifecycle ordering,
initializer error and panic handling, cross-platform builds, and steady-state
source/allocation review.

### AGT-COMP-001 — Static composite lifecycle

`CompositeAgent` MUST reject an empty Agent collection, construct its role
once as `[role-one,role-two]`, attempt every sub-agent startup and cleanup,
and return ordered aggregate recoverable errors.

Verification intent (informative): construction, exact role, lifecycle order,
and multiple-error tests.

### AGT-COMP-002 — Static composite duty cycle

`CompositeAgent::do_work` MUST advance its cursor before each sub-agent call,
resume after a failing sub-agent on the next call, reset after a full pass,
and combine successful work with wrapping `i32` addition without allocating.

Verification intent (informative): cursor, failure sequence, overflow, and
allocation tests.

### AGT-IDLE-001 — Idle strategy contract

`IdleStrategy` MUST support a work-count call, one idle step, reset, a static
alias, positive-work reset where applicable, zero/negative selection of the
idle path, and single-runner ownership of stateful implementations.

Verification intent (informative): trait and work-count tests.

### AGT-IDLE-002 — Backoff strategy

`BackoffIdleStrategy` MUST use alias `backoff`, Java defaults of 10 spins, 5
yields, 1,000 ns minimum park, and 1,000,000 ns maximum park, while preserving
Java's not-idle, spinning, yielding, parking, transition-boundary, and reset
behavior throughout the supported non-negative Rust duration domain.

The Java-compatible constructor domain is non-negative spin and yield counts
no greater than `i64::MAX` and minimum and maximum park durations no greater
than `i64::MAX / 2` nanoseconds. Larger Rust inputs are safe extensions but do
not carry a Java-equivalence claim.

Verification intent (informative): exact internal state traces and public
default/reset tests.

### AGT-IDLE-003 — Stateless strategies

`BusySpinIdleStrategy`, `NoOpIdleStrategy`, and `YieldingIdleStrategy` MUST
use aliases `spin`, `noop`, and `yield` while preserving Java's positive,
zero, and negative work-count behavior.

Verification intent (informative): separate component tests.

### AGT-IDLE-004 — Sleeping strategies

The sleeping strategies MUST make `SleepingIdleStrategy` use alias
`sleep-ns`, default to 1,000 ns, and park for zero/negative work; make
`SleepingMillisIdleStrategy` use alias `sleep-ms`, default to 1 ms, and sleep
for zero/negative work; and perform neither action for positive work.

Verification intent (informative): defaults, aliases, work counts, and
zero-duration tests.

### AGT-IDLE-005 — Controllable strategy

`ControllableIdleStrategy` MUST use alias `controllable`; acquire-observe a
release-published raw `i32` mode; map 1 to no-op, 2 to spin, 3 to yield, and 4
to a 1,000 ns park; and park for mode 0 or any unknown value.

Verification intent (informative): raw-mode, publication, alias, and
work-count tests.

### AGT-PORT-001 — Supported implementation profile

The Agent implementation MUST compile at Rust 1.85, execute its behavioral
tests on Linux, macOS, and Windows and its atomic tests on x86_64 and AArch64,
and perform zero allocations on successful steady-state invoker, runner,
composite, and idle paths after construction.

Verification intent (informative): CI, allocator instrumentation, stress
tests, source inspection, and benchmarks.

## Claim limits

Conformance does not claim Java source compatibility, binary compatibility,
wait-free Agent execution, forced cancellation, automatic core reservation,
affinity, real-time scheduling, or cross-language performance equivalence.
Shared-memory facilities and `DynamicCompositeAgent` remain outside scope.
