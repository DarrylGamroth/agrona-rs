# Agent verification evidence

> Maintainer verification record. For package usage, see the
> [User Guide](../../USER_GUIDE.md).

## Scope and baseline

- Requirements: `AGT-CORE-001` through `AGT-PORT-001` in
  [`AGENT_SPEC.md`](AGENT_SPEC.md).
- Agrona Java behavioral baseline:
  `d4a47c67258f85b39910c4999da346ead655b736`.
- Aeron C native comparison:
  `e44cd27a3b357c27ad37f6107a957f46d95552ac`.
- Verified implementation:
  `3b59551cfa6598c3c3fbcdf0d90d90f31891ef62`.
- GitHub Actions:
  [run 30312017451](https://github.com/DarrylGamroth/agrona-rs/actions/runs/30312017451).
- Local environment: Debian Linux 6.12.57, x86_64,
  `rustc 1.93.0 (254b59607 2026-01-19)`.
- MSRV check: Rust 1.85.0.

This document combines local x86_64 acceptance with native GitHub-hosted
x86_64 and AArch64 evidence for Linux, macOS, and Windows.

## Requirement coverage

| Requirement | Implementation and local evidence |
|---|---|
| `AGT-CORE-001` | Object-safe, mutable, `Send` Agent contract and default lifecycle in `src/agent/agent.rs`; public contract tests pass. |
| `AGT-ERR-001` | Typed recoverable failure and expected/unexpected termination remain separate; invoker and runner handling-matrix tests pass. |
| `AGT-ERR-002` | Relaxed wrapping error counter and owner-local handler; lifecycle, ordinary-error, continuation, and handler-panic tests pass. |
| `AGT-INV-001` | Caller-owned, internally unlocked state machine; start-once, close-once, error, termination, and panic tests pass. |
| `AGT-RUN-001` | Named dedicated thread, serialized lifecycle, and `idle(do_work())` loop; allocation instrumentation reports no steady-state allocations. |
| `AGT-RUN-002` | Release/acquire cooperative stop, unpark, ownership-preserving spawn failure, structured panic return, and retry diagnostics all pass locally. |
| `AGT-COMP-001` | Non-empty construction, exact role, all-agent lifecycle attempts, and ordered aggregation pass. |
| `AGT-COMP-002` | Pre-call cursor advance, resume-after-error, reset, wrapping `i32` sum, and allocation behavior pass. |
| `AGT-IDLE-001` | Work-count, reset, idle-step, and alias contracts are exercised per component. |
| `AGT-IDLE-002` | Java defaults and exact private backoff state transitions, boundaries, capped doubling, and reset pass. |
| `AGT-IDLE-003` | Busy-spin, no-op, and yielding aliases and positive/zero/negative work counts pass. |
| `AGT-IDLE-004` | Nanosecond park and millisecond sleep defaults, aliases, and work counts pass. |
| `AGT-IDLE-005` | Raw and typed controllable modes use release stores and acquire loads; aliases and work-count behavior pass. |
| `AGT-PORT-001` | Rust 1.85, native Linux/macOS/Windows, native x86_64/AArch64, native-atomic compile guard, allocation test, and benchmark checks pass. |

## Concurrency and progress audit

The worker calls Agent and idle callbacks through unique mutable ownership.
Its steady-state loop has no mutex, channel operation, or allocation. The
zero-capacity channel in `AgentRunner::start_with_builder` exists only until
the OS thread is successfully created and ownership has transferred.

`request_stop` release-stores `false`; the worker acquire-loads the flag on
every loop. It also unparks the worker so park-based strategies do not wait
for their timeout. `is_running` and `is_closed` use acquire loads. The
controllable idle handle release-stores its raw mode, and the strategy
acquire-loads it. These protocols are valid under both x86_64 and AArch64
memory models and do not depend on x86 store ordering.

The process-local error count uses relaxed `AtomicI64` operations because it
does not publish other state. `src/agent/mod.rs` rejects targets without
native 64-bit atomics, preventing an unseen lock-based fallback.

These are coordination claims, not a wait-free claim for a complete Agent
operation. Agent code, handlers, OS scheduling, yielding, parking, sleeping,
and application-owned wakeups may block or take unbounded time.

## Commands and local results

The following commands pass:

```text
cargo fmt --all --check
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features
cargo llvm-cov --workspace --all-features --summary-only
```

The complete crate reports 94.66% line coverage locally. Agent tests are kept
in separate component files, with private unit tests used only for exact
backoff state inspection.

## Local benchmark snapshot

`cargo bench --bench agents` ran 10,000,000 closed-loop iterations on the
shared local x86_64 host:

| Operation | Result |
|---|---:|
| `AgentInvoker::invoke` | 0.230 ns/op |
| `CompositeAgent::do_work` with two parts | 5.018 ns/op |
| `NoOpIdleStrategy::idle` | 0.235 ns/op |
| `BusySpinIdleStrategy::idle` | 15.120 ns/op |
| `AgentRunner::do_work` | 0.352 ns/op |

The table reports amortized closed-loop throughput cost rather than an
operation-latency distribution. The runner figure also amortizes thread
creation and join over the full run. These numbers are compiler-optimization
regression evidence, not latency promises: the host was not isolated, no CPU
affinity was set, and no dedicated-core or power-policy claim is made.

## GitHub CI results

GitHub Actions run `30312017451` passed at verified revision `3b59551`:

| Job | Result |
|---|---|
| Format, lint, and document | Passed |
| Test stable on Linux x86_64 | Passed |
| Test stable on Linux AArch64 | Passed |
| Test stable on macOS AArch64 | Passed |
| Test stable on Windows x86_64 | Passed |
| Check Rust 1.85 MSRV | Passed |
| Measure and upload coverage to Codecov | Passed |

The `AGENT-READY` evidence gate is closed.
