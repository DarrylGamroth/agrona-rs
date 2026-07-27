# Upstream Agrona

agrona-rs is an unofficial Rust port of selected components from
[Agrona](https://github.com/aeron-io/agrona).

The initial repository skeleton was designed against:

- repository: `https://github.com/aeron-io/agrona`
- commit: `d4a47c67258f85b39910c4999da346ead655b736`
- commit date: 2026-07-26

This commit records the source reviewed during initial design; it does not imply
that every component at that revision has been ported or verified.

## Implemented component mapping

The Clock implementation maps to these Agrona sources at the recorded
revision:

- `EpochClock`, `EpochMicroClock`, `EpochNanoClock`, and `NanoClock`;
- `SystemEpochClock`, `SystemEpochMicroClock`, `SystemEpochNanoClock`,
  `SystemNanoClock`, and `HighResolutionClock`;
- `CachedEpochClock` and `CachedNanoClock`; and
- `OffsetEpochNanoClock`.

The corresponding Rust implementation is under `src/clock/`. Behavioral and
allocation evidence is recorded in `docs/CLOCK_EVIDENCE.md`.

## Reference hierarchy

Agrona Java at the commit above is the normative behavioral reference for
selected ports.

The Agent design also reviews Aeron C at commit
`e44cd27a3b357c27ad37f6107a957f46d95552ac` as an implementation reference for
native ownership, thread lifecycle, atomic stop publication, and idle
primitives. Aeron C does not override Agrona Java behavior or defaults.

The sibling Julia packages are examples of language-specific ports:

- `Clocks.jl` commit `1b705421648b70b6d171f8a95d6c2d16c2444d0b`;
- `Agent.jl` commit `2e9f276fb6e7b573dda439a18f00aa80c0b3d69a`.

They are not normative references, test oracles, or sources of required edge
behavior.

## Porting policy

- Preserve externally visible Agrona behavior when compatibility is claimed.
- Preserve the progress class of each selected operation; never replace a
  Java lock-free path with a Rust mutex or lock.
- Prefer Rust ownership, lifetimes, traits, `Result`, and split producer/consumer
  handles over Java-shaped APIs.
- Record and test intentional Rust adaptations where Java mechanics have no
  direct safe equivalent.
- Retain the original copyright notice in substantially derived source files and
  mark them as modified for Rust.
- Record the upstream source and revision when a component becomes implemented.
- Require differential tests before claiming Java/Rust binary compatibility.
