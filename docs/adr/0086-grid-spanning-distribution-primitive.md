# ADR-0086: Order-independent Grid spanning distribution primitive

## Status

Accepted.

## Context

ADR-0085 introduced explicit Grid track sizing state with separate base sizes and growth limits. That state is necessary but not sufficient for spanning items.

CSS Grid's intrinsic spanning distribution requires each accommodated item to compute its own incurred increases against the same pre-round track state, while each affected track retains the largest planned increase seen in the round. The planned increases are applied only after all items in that round have been considered; this avoids item-order-dependent results.

Rarog's current retained Grid item contribution is still intentionally narrower than the specification's full contribution taxonomy. It does not yet distinguish every minimum, min-content and max-content contribution needed to wire the spanning phases into CSS-visible layout.

## Decision

Rarog adds a layout-internal spanning distribution primitive that is not yet called by the CSS Grid fragment builder.

The primitive accepts synthetic size contributions with:

- source node identity;
- start track;
- span;
- already-selected size contribution.

For the currently representable fixed/auto track set, the primitive:

1. validates finite non-negative contribution geometry;
2. subtracts the current base sizes of every spanned track plus internal gaps from the contribution to obtain extra space;
3. treats supported `auto` tracks in the span as affected tracks;
4. distributes the extra space equally across affected tracks, freezing a track when its finite growth limit is reached;
5. records per-item incurred increases separately from track state;
6. retains the maximum incurred increase for each track as that round's planned increase;
7. applies planned increases only in a separate operation after the round.

The distribution therefore does not mutate sizing state while individual spanning items are being considered.

If equal-share arithmetic underflows to zero for subnormal finite space, the bounded primitive terminates without further growth rather than looping.

The helper is crate-internal and does not enable spanning Grid layout, new CSS syntax or a claim of full section 12.5.1 conformance.

## Consequences

Rarog now has an order-independent mechanism suitable for the later intrinsic spanning phases once contribution kinds are represented accurately.

Gap accounting and growth-limit freezing live in one layout-owned primitive instead of being recreated by CSS-facing layout code.

Current Grid rendering remains unchanged.

## Deferred

Before this primitive can drive CSS-visible spanning intrinsic sizing, later slices still need:

- explicit minimum/min-content/max-content contribution classes;
- a bounded mapping from retained intrinsic measurements to the correct contribution class for each sizing phase;
- phase ordering by increasing item span;
- the remaining non-affected-track and beyond-limit distribution rules required when richer min/max track functions exist;
- intrinsic growth-limit resolution and the infinitely-growable state;
- flexible tracks and `fr`;
- `minmax()`, min-content/max-content track functions, `fit-content()`, `repeat()` and implicit tracks.
