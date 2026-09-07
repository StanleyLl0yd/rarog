# ADR-0096: Production intrinsic spanning for the bounded fixed/auto Grid subset

## Status

Accepted.

## Context

ADR-0095 validated the composed intrinsic spanning algorithm in the layout test harness. Production CSS Grid still called the older non-spanning state resolver, so any item spanning more than one track while touching an intrinsic `auto` track failed closed and the FragmentBuilder discarded that Grid geometry.

The production CSS path already supplies the semantic contribution pair required by the validated model: minimum contributions establish base sizes and max-content contributions establish growth limits. It also supplies the declared gutter and definite space when that space is genuinely known.

Other callers still use compatibility projections such as max-content for both base and growth contributions. Those projections are not evidence that the complete spanning phase ordering is valid for that mode.

## Decision

Rarog promotes the ADR-0095 composed resolver and growth-limit helpers from test-only code into the layout implementation.

`resolve_intrinsic_tracks_with_space` selects the composed spanning resolver only when:

- the base contribution kind is `Minimum`; and
- the growth-limit contribution kind is `MaxContent`.

That is the semantic path used by production CSS Grid in the current bounded fixed/`auto` subset.

All other contribution-pair combinations continue through the existing non-spanning resolver. In particular, the public legacy max-content-only adapter keeps returning `UnsupportedIntrinsicSpan` for intrinsic multi-track spans rather than silently presenting a different algorithm as standards-correct.

The composed resolver retains the ADR-0095 rules:

- single-span minimum and max-content initialization precedes wider spans;
- wider spans are processed in increasing span order;
- declared fixed gutters participate in span occupancy;
- minimum contributions grow base sizes;
- min-content and max-content phases grow limits with the temporary infinitely-growable handoff;
- planned increases remain order-independent within each span round;
- remaining infinite auto growth limits close to their base sizes before final maximize/stretch phases.

## Validation

A production resolver regression covers two `auto` columns, a spanning intrinsic contribution, a declared gutter and definite free space.

A FragmentBuilder regression exercises CSS-visible geometry with:

- two `auto` columns;
- a 4px declared column gap;
- an item spanning both columns;
- intrinsic text whose min-content and max-content sizes differ;
- CSS-visible spanning geometry produced after intrinsic track sizing.

The legacy max-content-only spanning failure regression remains unchanged.

## Consequences

CSS Grid can now produce bounded intrinsic spanning geometry for the supported fixed/`auto` subset without removing the conservative failure boundary from compatibility-only APIs.

This does not add flexible tracks, fit-content, implicit tracks, `minmax()`, `repeat()`, named lines, subgrid or the wider automatic-minimum rules needed for broader Grid completeness.

## Deferred

Fractional and richer maximum-track sizing remain the next Grid standards workstream.
