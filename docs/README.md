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

The remaining documents in this directory are specifications, plans,
traceability ledgers, and delivery evidence for maintaining compatibility
with upstream Agrona. They are not required to use the crate.

- [Porting plan](PORTING_PLAN.md)
- [Ecosystem review](ECOSYSTEM_REVIEW.md)
- [Agent specification](AGENT_SPEC.md)
- [Agent implementation plan](AGENT_IMPLEMENTATION_PLAN.md)
- [Agent evidence](AGENT_EVIDENCE.md)
- [Clock specification](CLOCK_SPEC.md)
- [Clock evidence](CLOCK_EVIDENCE.md)
- [Test matrix](TEST_MATRIX.md)
