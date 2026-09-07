# ADR-0095: Composed intrinsic spanning rounds for auto Grid tracks

## Status

Accepted.

## Context

Rarog already has the pieces needed for bounded intrinsic Grid spanning:

- semantic minimum / min-content / max-content item contributions;
- order-independent planned base-size increases;
- increasing-span base-size rounds;
- explicit track base sizes and growth limits;
- definite-space maximization and final auto-track stretch;
- Grid track-group content distribution.

Production layout still fails closed when an item spans multiple tracks including an intrinsic `auto` track.

That fail-closed boundary remains necessary because the earlier synthetic spanning primitive only composed base-size rounds. CSS Grid intrinsic spanning also depends on when growth limits become finite and on the temporary “infinitely growable” state between the intrinsic-maximum and max-content-maximum phases.

The existing non-spanning resolver also legitimately used a zero gutter when applying its span-1 base contribution, but a real multi-track span must account for the declared fixed gutter.

## Decision

Rarog adds a test-harness semantic resolver for the currently supported fixed / `auto` track subset.

The composed resolver and its growth-limit helpers are compiled under `cfg(test)` in this ADR. Production Grid geometry remains on the existing fail-closed boundary; the next rollout slice will promote the validated model into production code when it becomes reachable from layout.

### Single-span initialization

Before any multi-track span is considered, single-span items establish:

- the largest minimum contribution as each `auto` track's base size;
- the largest max-content contribution as its finite growth limit.

This ordering matters: later spanning base-size distribution must see growth limits established by earlier single-span items.

### Increasing-span composed rounds

Multi-track items are grouped by increasing span.

For each span size, Rarog performs the bounded subset of the CSS Grid intrinsic track-sizing phases in this order:

1. distribute minimum contributions into `auto` track base sizes;
2. clamp finite growth limits so they are at least the resulting base size;
3. distribute min-content contributions into intrinsic `auto` growth limits;
4. mark any affected `auto` track whose growth limit changed from infinite to finite as infinitely growable for the immediately following phase;
5. distribute max-content contributions into `auto` growth limits while honoring that temporary infinitely-growable state.

Only fixed and `auto` tracks exist in this bounded resolver. There are no flexible or fit-content tracks, so the general Grid distribution algorithm simplifies without changing the result for this subset.

### Growth-limit distribution

When calculating the occupied growth-limit size of a span:

- a finite growth limit contributes at least the track base size;
- an infinite growth limit contributes the track base size;
- fixed gutters between the spanned tracks are included.

For a growth-limit phase, tracks whose growth limit is still infinite or is temporarily marked infinitely growable receive the up-to-limit distribution first.

If no such track exists, remaining space may grow the affected intrinsic-max `auto` tracks beyond their current finite growth limits, as required by the intrinsic growth-limit distribution step.

Planned increases remain per-track maxima across items in the same round and are applied only after every item in that round is considered, preserving order independence.

### Base-size distribution beyond growth limits

The existing base-size planner already freezes tracks at finite growth limits while other affected tracks can still grow.

For the supported `auto` subset, if all affected intrinsic-max tracks reach those limits and contribution space still remains, the planner now distributes the remainder equally beyond those limits. The subsequent semantic round raises finite growth limits to at least the new base sizes.

Applying a planned base-size increase now applies the already-resolved plan exactly; limit handling belongs to the planning phase rather than being re-applied during mutation.

### Finalization

After all span groups are processed, any remaining infinite `auto` growth limit is closed to the track's base size.

## Validation

The primitive tests cover:

- the CSS Grid specification's representative two-`auto`-track case where a pre-sized first track remains at growth limit 10 while the second track becomes 90;
- fixed gutters;
- mixed fixed / `auto` spans;
- base growth beyond previously finite intrinsic growth limits;
- item-order independence.

## Consequences

Rarog now has a composed, gap-aware intrinsic spanning state model validated in the layout test harness for the current fixed / `auto` subset.

The old `UnsupportedIntrinsicSpan` production boundary is intentionally retained in this slice. The next integration slice can switch production Grid sizing to this resolver with focused CSS-visible regressions instead of mixing algorithm construction with rollout.

The generic legacy single-span compatibility resolver is unchanged.

## Deferred

This ADR does not add:

- flexible tracks or `fr`;
- fit-content limits;
- min-content / max-content container constraint reruns;
- richer automatic minimum-size eligibility;
- implicit tracks;
- `minmax()`, `repeat()`, named lines or subgrid;
- production CSS-visible intrinsic spanning.

Those remain separate bounded slices.
