# ADR-0064: Flex display integration boundary

## Status

Accepted.

## Context

ADR-0063 introduced a Rarog-owned bounded single-line flex-row placement primitive but deliberately left CSS-facing `display: flex` parsing and layout dispatch disconnected until that primitive existed.

R3 now needs the smallest truthful end-to-end Flexbox slice without claiming unsupported sizing, alignment, wrapping or anonymous-item behavior.

## Decision

Rarog accepts `display: flex` as a distinct computed-style state and dispatches flex containers through the existing layout-owned single-line row primitive.

The first integrated slice is deliberately bounded:

- direct box children participate only when both content `width` and `height` are explicitly resolved by the current CSS model;
- border and padding are included in each item's row base size while margins remain explicit placement edges;
- source order is preserved;
- whitespace-only direct text nodes are ignored;
- flex containers and direct flex items form margin-collapse boundaries for the current block-layout model;
- nested fixed-size flex containers may recursively use the same bounded row path;
- incremental geometry changes to a flex item relayout the containing row so retained siblings cannot keep stale horizontal positions.

If a direct item requires unsupported auto/intrinsic flex-base-size resolution, contains non-whitespace anonymous text, has unsupported negative margins, or produces invalid/non-finite placement geometry, this slice fails closed by producing no flex-item fragments for that container. It does not silently reinterpret the container as ordinary block flow.

## Consequences

This establishes a real CSS -> computed style -> layout dispatch path for `display: flex` while keeping the implementation honest about its current standards boundary.

The fail-closed behavior is intentionally temporary. Later R3 slices should replace those unsupported cases with measured Flexbox behavior rather than preserving the empty result as compatibility behavior.

## Deferred

The following remain explicit R3 follow-ups:

- flex base-size resolution from auto/intrinsic content;
- `flex-grow` and `flex-shrink`;
- min/max main-size participation in flexible sizing;
- `justify-content`, `align-items`, `align-self` and baseline alignment;
- `gap`;
- wrapping;
- reverse directions and writing modes;
- negative margins;
- non-whitespace anonymous flex items and broader Flexbox edge cases.
