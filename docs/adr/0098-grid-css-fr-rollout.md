# ADR-0098: Bounded CSS fr rollout

## Status

Accepted.

## Context

ADR-0097 introduces layout-owned fractional track metadata and definite-space flex-fraction expansion while deliberately leaving CSS `fr` syntax rejected. This separates parser acceptance from the algorithm that gives accepted syntax meaning.

The current layout foundation is sufficient for flexible tracks when available space on the axis is definite and no intrinsic item spans a flexible track. Indefinite flexible sizing and flexible intrinsic spanning require additional CSS Grid phases and must not be approximated.

## Decision

The CSS computed track model adds `GridTrackSize::Fraction(f32)`.

The bounded whitespace-separated `grid-template-columns` / `grid-template-rows` parser accepts finite, non-negative `fr` dimensions such as `1fr`, `0.5fr` and ASCII-case-insensitive `2FR`. Invalid or unsupported forms continue to reject the whole declaration.

The FragmentBuilder maps computed fractional tracks into layout-owned `GridTrackSizing::Fraction` metadata. Fractional rows participate in natural block contribution measurement in the same production path as intrinsic `auto` rows.

The semantic resolver fails closed with `IndefiniteFlexibleTracks` whenever a fractional track is present but the available space for that axis is indefinite. This keeps automatic-height flexible rows from silently using an incomplete approximation.

Items spanning a flexible track retain the ADR-0097 `UnsupportedIntrinsicSpan` boundary.

## Validation

Parser tests cover accepted fractional factors, ASCII case-insensitive units and rejection of negative, non-finite, missing-factor and malformed forms.

CSS-visible layout tests cover:

- `1fr 3fr` columns in a definite 110px content box with a 10px declared gutter, producing 25px and 75px tracks;
- `1fr 3fr` rows in a definite 100px block axis, producing 25px and 75px tracks;
- the existing fixed/`auto` Grid behavior remains under the same regression suite.

A layout-unit regression verifies that indefinite flexible tracks return the explicit fail-closed error.

## Consequences

Common definite-width `fr` columns and definite-height `fr` rows become CSS-visible without coupling parser representation to the layout algorithm.

This slice does not claim complete CSS Grid flexible sizing.

## Deferred

Deferred work includes indefinite-space flex fractions, intrinsic items spanning flexible tracks, `minmax()`, fit-content, percentage tracks, implicit tracks, repeat syntax, named lines and subgrid.
