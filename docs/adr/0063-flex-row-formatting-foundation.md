# ADR-0063: First flex row formatting foundation

Status: Accepted

## Context

R3 starts modern layout work with flexbox. Rarog's existing layout path understands block and inline flow but has no explicit flex formatting algorithm. Adding `display: flex` to CSS before such an algorithm exists would create a dangerous silent fallback where flex content is rendered as ordinary block flow.

The first slice therefore needs a layout-owned representation and deterministic geometry algorithm that can be tested independently before computed-style and layout-tree dispatch are connected.

## Decision

`rarog-layout` owns the first flex formatting primitive in `flex.rs`.

The initial algorithm is deliberately limited to a single, horizontal, source-order row of items whose border-box base sizes are already resolved. It places items along the main axis, accounts for non-negative margins, reports content extent and reports main/cross-axis overflow without silently shrinking, wrapping or redistributing items.

The boundary uses only Rarog-owned `LayoutNodeId`, `Point`, `Rect`, `Size` and `EdgeSizes` values. No CSS parser AST or future graphics/compositor type enters the algorithm.

Invalid or unsupported geometry is rejected explicitly. Available sizes and item base sizes must be finite and non-negative; margins must be finite. Negative margins are rejected in this first slice rather than approximated incorrectly.

This slice does **not** yet parse or dispatch `display: flex`. CSS integration follows only after the layout primitive is established and tested, preventing unsupported flex content from being silently treated as standards-correct block layout.

## Deferred flex behavior

The following remain explicit follow-up work:

- flex base-size resolution from CSS/intrinsic sizing;
- `flex-grow` and `flex-shrink` free-space distribution;
- min/max main-size clamping during flexible sizing;
- `justify-content`, `align-items`, `align-self` and baseline alignment;
- gaps;
- wrapping and multi-line flex containers;
- reverse directions and writing-mode interactions;
- negative margins;
- anonymous flex items and broader spec edge cases.

## Consequences

R3 begins with a small deterministic algorithm that can be integrated incrementally and measured independently. Until the next integration slice, Web content does not gain `display:flex` semantics merely because the lower-level row primitive exists.
