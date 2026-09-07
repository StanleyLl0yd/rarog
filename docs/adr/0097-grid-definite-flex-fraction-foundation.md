# ADR-0097: Layout-owned definite-space flex fraction foundation

## Status

Accepted.

## Context

Rarog's bounded Grid sizing model currently owns only fixed and `auto` tracks. The next standards step is flexible `fr` sizing, but exposing parser syntax before the track algorithm exists would turn accepted CSS into incomplete geometry.

CSS Grid sizes flexible tracks after Maximize Tracks and before stretching `auto` tracks. For definite free space, the flex fraction is found from leftover space and flex-factor sums. If a flexible track's hypothetical fraction would make it smaller than its already-established base size, that track is treated as inflexible and the fraction calculation restarts.

Bare `fr` tracks also have an automatic minimum. Their content can therefore establish a non-zero base size before flexible expansion.

## Decision

Rarog adds `GridTrackSizing::Fraction(f32)` as layout-owned metadata without changing the CSS parser in this slice.

A fractional track:

- initializes with base size zero and an infinite growth limit;
- accepts a finite, non-negative flex factor;
- receives the minimum intrinsic contribution of a single-span item as its base size;
- closes any remaining infinite growth limit to its base size after intrinsic sizing so Maximize Tracks does not consume flexible space;
- participates in a new Expand Flexible Tracks phase after Maximize Tracks and before `auto` stretch.

For definite available space, Rarog finds the flex fraction by:

1. removing declared fixed gutters from the space to fill;
2. subtracting the base sizes of non-flexible and already-frozen flexible tracks;
3. summing active flex factors, with a minimum denominator of one;
4. computing the hypothetical flex fraction;
5. freezing any active flexible track whose factor times that fraction would fall below its current base size;
6. restarting until no additional track freezes.

The resulting fraction only increases flexible base sizes; it never shrinks an intrinsic base.

## Bounded intrinsic boundary

The current fixed/`auto` composed spanning rounds remain unchanged.

A multi-track item that crosses any fractional track still returns `UnsupportedIntrinsicSpan`. CSS Grid requires a dedicated intrinsic phase for items spanning flexible tracks, including factor-weighted distribution and special automatic-minimum behavior. That phase is intentionally not approximated here.

Single-span items in fractional tracks are supported by the layout-owned semantic resolver so the next CSS rollout can be limited to a well-defined non-spanning subset.

## Validation

Tests cover:

- fixed tracks and declared gutters participating in leftover-space calculation;
- flex-factor sums below one leaving unrequested space unfilled;
- restart behavior when a flexible track's existing base size exceeds its hypothetical share;
- a single flexible track item establishing an intrinsic minimum before expansion;
- the retained fail-closed boundary for intrinsic spans crossing flexible tracks.

## Consequences

The layout crate now owns the `fr` sizing primitive independently of parser representation. The CSS parser continues rejecting `fr` in this slice.

The next rollout can add bounded `fr` syntax and map it to `GridTrackSizing::Fraction` without combining parser work with algorithm construction.

## Deferred

Deferred work includes flexible intrinsic spans, indefinite-space flex fractions, `minmax()`, fit-content, percentage tracks, implicit tracks, repeat syntax, named lines and subgrid.
