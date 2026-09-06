# ADR-0085: Explicit Grid track sizing state

## Status

Accepted.

## Context

ADR-0082 added the first bounded content-driven Grid track sizing slice. Its resolver could represent only a resolved `GridTrack { base_size }`, which was sufficient while every intrinsic contribution was restricted to a single `auto` track.

The CSS Grid track-sizing algorithm needs more state before spanning intrinsic contributions or flexible tracks can be implemented correctly. In particular, a track has both a base size and a growth limit, and later sizing phases may grow one or the other under different rules.

Adding spanning distribution directly to the one-number geometry representation would force Rarog either to invent an approximation or to repeatedly reconstruct information that the algorithm conceptually owns.

## Decision

Rarog introduces a layout-internal `GridTrackSizingState` with:

- `base_size`;
- `growth_limit`.

The growth limit is represented explicitly as either:

- `Finite(f32)`;
- `Infinite`.

Initialization for the currently supported track functions is:

- fixed track: base size = fixed size, growth limit = the same fixed size;
- `auto` track: base size = 0, growth limit = infinite.

The existing single-span intrinsic sizing phase now operates on this state and materializes the public `GridTrack` geometry only after sizing is complete.

Growing a base size is constrained by the state's growth limit. Under the currently supported syntax this does not change behavior: fixed tracks do not grow from intrinsic contributions and `auto` tracks retain an infinite growth limit.

The new state is layout-owned and crate-internal. Public Grid geometry remains the already-resolved `GridTrack`, and no new CSS syntax is enabled by this decision.

## Consequences

The current Grid output is unchanged, while later sizing phases now have an explicit place to carry growth-limit state.

Future spanning-item work can be implemented as a sizing phase over state rather than by mutating final geometry ad hoc.

The CSS parser and computed-style contracts remain independent from layout algorithm state.

## Deferred

This ADR does not implement:

- distribution of intrinsic contributions across spanning tracks;
- temporary planned increases used for order-independent spanning distribution;
- finite intrinsic growth limits derived from additional max track-sizing functions;
- flexible track sizing and `fr`;
- `minmax()`, min-content/max-content track functions or `fit-content()`;
- `repeat()` or implicit tracks.

Those features require additional bounded sizing phases and remain unsupported until implemented explicitly.
