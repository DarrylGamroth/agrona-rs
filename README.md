agrona-rs
=========

[![GitHub release](https://img.shields.io/github/v/release/DarrylGamroth/agrona-rs)](https://github.com/DarrylGamroth/agrona-rs/releases)
[![GitHub](https://img.shields.io/github/license/DarrylGamroth/agrona-rs.svg)](LICENSE)

[![Actions Status](https://github.com/DarrylGamroth/agrona-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/DarrylGamroth/agrona-rs/actions)
[![Codecov](https://codecov.io/gh/DarrylGamroth/agrona-rs/graph/badge.svg)](https://codecov.io/gh/DarrylGamroth/agrona-rs)

agrona-rs provides data structures and utilities commonly needed when
building high-performance applications in Rust. It is an unofficial,
idiomatic port of selected components from
[Agrona](https://github.com/aeron-io/agrona), which is used by the
[Aeron](https://github.com/aeron-io/aeron) messaging system.

The port preserves Agrona Java behavior where compatibility is claimed while
using Rust ownership, traits, typed errors, and explicit thread handles.
Agrona Java is the normative behavioral reference; Aeron C is an additional
native implementation reference for Agent ownership, atomics, and idle
primitives.

Utilities Include:

* Clocks - Epoch and monotonic providers, single-writer cached clocks, and an
  offset epoch nanosecond clock.
* Simple Agent framework - Mutable duty cycles with lifecycle callbacks,
  typed recoverable errors, and expected or unexpected termination.
* Agent execution - Caller-driven invocation and dedicated named OS threads
  with cooperative shutdown.
* Agent composition - Static composition of heterogeneous Agents with exact
  Agrona cursor and lifecycle behavior.
* Idle strategies - Backoff, busy-spin, controllable, no-op, nanosecond
  sleeping, millisecond sleeping, and yielding strategies.

`DynamicCompositeAgent`, shared-memory counters and controls, buffers, queues,
and the other Agrona utility families are not currently implemented. The
runner's steady-state loop introduces no mutex, channel operation, or heap
allocation. Shutdown remains cooperative, so blocking Agent code must provide
an application-owned wakeup mechanism.

For the selected scope and compatibility decisions see the
[Porting Plan](docs/PORTING_PLAN.md). Normative behavior and verification are
recorded in the [Agent specification](docs/AGENT_SPEC.md),
[Agent evidence](docs/AGENT_EVIDENCE.md), and
[Clock evidence](docs/CLOCK_EVIDENCE.md). Exact upstream revisions are in
[UPSTREAM.md](UPSTREAM.md).

Use
---

Releases are published only through
[GitHub Releases](https://github.com/DarrylGamroth/agrona-rs/releases), not
crates.io or docs.rs. Depend on a release tag directly from GitHub:

```toml
[dependencies]
agrona = { git = "https://github.com/DarrylGamroth/agrona-rs", tag = "v0.1.0" }
```

Build API documentation locally with `cargo doc --open`.

Build
-----

### Rust Build

Build the project with [Cargo](https://doc.rust-lang.org/cargo/).

Rust 1.85 is the minimum supported compiler. The project uses Rust 2024
edition and is tested on stable Rust for Linux x86_64, Linux AArch64, macOS
AArch64, and Windows x86_64.

Full build and test:

    $ cargo test --workspace --all-targets --all-features

Formatting, lint, documentation, and MSRV checks:

    $ cargo fmt --all --check
    $ cargo clippy --workspace --all-targets --all-features -- -D warnings
    $ RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
    $ cargo +1.85.0 check --workspace --all-targets --all-features

License (See LICENSE file for full license)
-------------------------------------------

Copyright 2026 Rubus Technologies Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
