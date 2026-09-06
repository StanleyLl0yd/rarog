# ADR-0082: Bounded intrinsic Grid auto-track sizing

## Status

Accepted.

## Context

Rarog's first Grid slices established bounded explicit fixed tracks, explicit-grid auto-placement and item self-alignment while preserving a compact `ComputedStyle: Copy` representation and keeping parser AST ownership out of layout.

The next useful Grid sizing step is content-driven `auto` tracks. Implementing the complete CSS Grid track-sizing algorithm in one change would also require spanning-item distribution, flexible tracks, min/max track sizing, implicit tracks and additional sizing phases. Approximating those semantics would make unsupported behavior appear standards-correct.

## Decision

The bounded computed Grid track list remains a fixed-capacity Copy value, but each explicit track is represented as `GridTrackSize::Fixed(f32)` or `GridTrackSize::Auto`.

CSS accepts `auto` alongside the existing finite non-negative pixel tracks. The existing eight-track-per-axis bound remains unchanged.

Layout converts computed tracks into independent Rarog-owned `GridTrackSizing` metadata. CSS parser types do not enter the Grid geometry primitive.

Placement resolves before intrinsic sizing. The first intrinsic sizing slice then follows these rules:

1. fixed tracks keep their resolved fixed size;
2. an `auto` track begins at zero;
3. only single-track-spanning items contribute to an `auto` track in this slice;
4. a column `auto` track grows to the largest participating item's max-content margin-box contribution;
5. columns are resolved before block-axis measurement;
6. a row `auto` track grows to the largest participating item's natural margin-box block size, measured with an isolated fragment builder at the already-resolved inline size;
7. fixed rows are not remeasured, and items that do not touch an `auto` row are not measured for row-track sizing;
8. an item spanning multiple tracks where the span includes an intrinsic `auto` track fails closed with `UnsupportedIntrinsicSpan` instead of using an invented distribution rule;
9. invalid or missing intrinsic contributions fail explicitly;
10. resolved intrinsic tracks are not clamped to the Grid container's available size. Existing overflow reporting remains responsible for recording track overflow.

The existing fixed Grid entry points remain available. `resolve_grid_placements` and `resolve_content_sized_tracks` expose the placement and bounded sizing phases separately so later sizing work can extend them without importing CSS syntax into layout.

## Consequences

Common explicit Grid layouts can now use content-driven `auto` columns and rows while preserving the existing Copy computed-style model.

Track sizing consumes retained layout intrinsic data and the same isolated natural-height measurement already used by Flex, avoiding a second text/content sizing model.

Fixed-track Grid behavior remains on the previous non-measuring path.

Unsupported multi-track intrinsic distribution remains visible and testable instead of being silently approximated.

## Deferred

Later Grid slices still own:

- intrinsic automatic sizing of non-stretch Grid items;
- intrinsic contribution distribution across spanning items;
- `fr` tracks;
- `minmax()`, min-content and max-content track functions;
- `repeat()`;
- implicit tracks and broader auto-flow;
- named lines/areas and negative lines;
- baseline alignment, auto margins and broader writing-mode interactions;
- subgrid and the remaining CSS Grid sizing algorithm.
