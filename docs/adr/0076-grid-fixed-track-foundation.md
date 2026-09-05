# ADR-0076: Fixed explicit Grid layout foundation

## Status

Accepted.

## Context

The R3 backlog calls for Rarog-owned Grid track/item metadata and a first measured Grid layout slice.

The current CSS `ComputedStyle` remains a compact `Copy` value. Arbitrary `grid-template-columns` / `grid-template-rows` lists do not fit that representation without either introducing parser-owned heap structures into computed style or inventing an undocumented fixed cap. Neither is an acceptable foundation.

Grid geometry should therefore be established independently before CSS integration, following the same ownership pattern used by the first Flexbox row primitive.

## Decision

Rarog adds a layout-owned Grid module with:

- `GridTrack` for an explicit fixed non-negative track base size;
- `GridItem` for explicit zero-based row/column starts and positive row/column spans;
- `GridPlacement` for the resulting grid-area rectangle;
- `GridLayout` for source-ordered placements, explicit-grid content extent and overflow flags;
- `GridLayoutError` for invalid origin/available size, tracks, gaps, spans, out-of-grid placement and finite-geometry overflow.

The first entry point is `layout_fixed_grid`.

This slice supports only explicit fixed rows and columns. Track positions are the prefix sum of fixed track sizes plus fixed gaps. A spanning grid area includes all covered tracks and the internal gaps between them.

Grid items are not reordered. Overlapping explicit placements are legal and remain in source order.

The primitive reports content width and height from explicit track geometry and flags overflow relative to the caller-provided available size.

All supplied geometry must remain finite. Track sizes and gaps must be non-negative. Row and column spans must be positive and remain inside the explicit grid. Unsupported or invalid geometry fails explicitly.

## Consequences

Rarog now has a CSS-independent Grid ownership boundary that can be expanded without coupling parser AST types to layout.

The primitive is useful for deterministic track/item geometry tests before CSS syntax, auto placement, intrinsic track sizing or item alignment are introduced.

The existing `ComputedStyle: Copy` contract remains unchanged.

## Deferred

Later R3 slices still own:

- CSS `display:grid` dispatch;
- the computed representation of explicit grid track lists;
- auto placement;
- `fr`, intrinsic, min/max and auto track sizing;
- grid item alignment;
- implicit tracks;
- named lines/areas;
- writing-mode interactions.
