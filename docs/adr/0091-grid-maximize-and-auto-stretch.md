# ADR-0091: Definite Grid maximize and auto-track stretch phases

## Status

Accepted.

## Context

ADR-0090 separates intrinsic Grid base sizes from growth limits, but the compatibility projection still materializes those intrinsic sizes directly.

CSS Grid track sizing has two later non-flex phases that matter before Rarog can remove that projection for definite Grid containers:

1. Maximize Tracks distributes positive free space to all track base sizes, freezing tracks when they reach their growth limits.
2. Stretch auto Tracks, when content distribution permits stretching, divides any remaining positive definite free space equally among tracks whose max track sizing function is `auto`.

These phases have different limit behavior. Maximize respects intrinsic growth limits; final auto-track stretch is an alignment expansion and can grow an `auto` track beyond its intrinsic max-content growth limit.

## Decision

Rarog adds a layout-internal finalization orchestrator for the currently supported fixed/`auto` track set.

The orchestrator accepts:

- resolved track sizing state;
- the corresponding bounded track sizing metadata;
- the used fixed gutter size;
- definite or indefinite available Grid space;
- the axis;
- whether auto-track stretch is enabled for the axis.

For definite available space:

### Maximize Tracks

Rarog computes positive free space as:

`available grid space - track base sizes - fixed gutters`, floored at zero.

It distributes that free space equally across all tracks while freezing tracks at their growth limits. Fixed tracks therefore remain fixed because their growth limit equals their base size. Intrinsic tracks may grow up to their resolved growth limits.

### Stretch auto Tracks

If stretch is enabled after maximization, Rarog recomputes remaining positive definite free space and divides it equally among tracks represented as `GridTrackSizing::Auto`.

This final stretch is not capped by the earlier intrinsic growth limit. No later sizing phase consumes that limit in the current bounded pipeline.

For indefinite available space, the orchestrator leaves resolved intrinsic state unchanged.

Invalid finite-space inputs fail closed.

The existing compatibility resolver calls the orchestrator with indefinite available space in this slice, so current CSS-visible Grid geometry is unchanged.

## Consequences

Rarog now has bounded implementations of the non-flex §12.6 Maximize Tracks and §12.8 Stretch auto Tracks behavior needed by the supported fixed/`auto` track subset.

Gutters participate in free-space accounting as fixed space.

The implementation does not introduce a second distribution algorithm for Maximize; it reuses the existing growth-limit-aware equal-distribution primitive.

The next integration slice can pass actual definite Grid content-box space and switch the supported non-spanning `auto` path from the historical max-content projection to minimum-base / max-content-growth / maximize / stretch phases.

## Deferred

This ADR does not yet:

- connect container `justify-content` / `align-content` semantics to the stretch flag;
- switch production Grid geometry to definite-space finalization;
- expand flexible tracks;
- implement indefinite-space min/max constraint reruns;
- enable intrinsic spanning items in CSS-facing layout;
- model additional intrinsic/flexible track sizing functions.

Those remain separate bounded slices.
