# Documentation

## User documentation

- [User Guide](USER_GUIDE.md) — installation, clocks, Agents, execution
  models, idle strategies, errors, and shutdown.
- [Clock example](../examples/clocks.rs) — epoch, monotonic, cached, and offset
  clock usage.
- [Agent runner example](../examples/agent_runner.rs) — a complete Agent
  lifecycle on a dedicated thread.

The User Guide is embedded as the crate-level rustdoc. Run `cargo doc --open`
to browse it together with the complete API reference.

## Maintainer documentation

Specifications, plans, traceability ledgers, and delivery evidence live under
[`design/`](design/). They maintain compatibility with upstream Agrona and
are not required to use the crate.

- Shared design:
  - [Porting plan](design/PORTING_PLAN.md)
  - [Ecosystem review](design/ECOSYSTEM_REVIEW.md)
  - [Test matrix](design/TEST_MATRIX.md)
- Agent design:
  - [Specification](design/agent/AGENT_SPEC.md)
  - [Implementation plan](design/agent/AGENT_IMPLEMENTATION_PLAN.md)
  - [Evidence](design/agent/AGENT_EVIDENCE.md)
  - [Traceability ledger](design/agent/agent_traceability.toml)
- Clock design:
  - [Specification](design/clock/CLOCK_SPEC.md)
  - [Evidence](design/clock/CLOCK_EVIDENCE.md)
  - [Traceability ledger](design/clock/clock_traceability.toml)
- Counter-reader design:
  - [Specification](design/counters/COUNTER_SPEC.md)
  - [Implementation plan](design/counters/COUNTER_IMPLEMENTATION_PLAN.md)
  - [Evidence](design/counters/COUNTER_EVIDENCE.md)
  - [Traceability ledger](design/counters/counter_traceability.toml)
