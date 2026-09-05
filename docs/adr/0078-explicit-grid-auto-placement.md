# ADR-0078: Bounded auto-placement inside the explicit Grid

## Status

Accepted.

## Context

#149 connected fixed explicit CSS Grid to the Rarog-owned Grid primitive, but every item required both row and column starts. CSS Grid commonly relies on auto-placement.

Creating implicit tracks or implementing the full Grid auto-placement algorithm would broaden the sizing surface before intrinsic and fractional track sizing exist.

## Decision

Rarog adds a layout-owned `GridPlacementRequest` alongside the existing concrete `GridItem`. Existing explicit APIs remain source-compatible.

A request contains an optional row start, optional column start and positive row/column spans.

Auto-placement is bounded to the already-existing explicit tracks:

1. validate the explicit Grid geometry and request spans;
2. resolve every request with both axes explicit and reserve its area for later auto placement;
3. keep explicit overlap legal;
4. resolve remaining requests in source order:
   - both axes auto: scan row-major from the first explicit cell;
   - fixed row and auto column: scan columns in that row;
   - auto row and fixed column: scan rows in that column;
5. a candidate must fit entirely inside the explicit tracks and avoid all reserved explicit areas and previously auto-placed areas;
6. if no candidate fits, return a bounded auto-placement failure instead of creating implicit tracks;
7. after every request resolves, delegate final geometry to the existing fixed Grid layout function in original source order.

Occupancy is represented as a list of occupied rectangular Grid areas rather than a rows×columns bitmap, so the public layout primitive does not acquire an accidental memory cost proportional to track-count multiplication.

CSS integration uses the existing computed `auto` row/column starts from #149; no new parser surface is required.

When auto-placement cannot resolve, fragment construction remains fail-closed while preserving the explicit Grid track extent.

## Consequences

Common row-major Grid markup can render without explicit placement on every child while Grid sizing remains strictly bounded to fixed explicit tracks.

Explicit overlap/source-order behavior is unchanged.

The old `GridItem` and `layout_fixed_grid` APIs remain available and unchanged.

## Deferred

Later R3 Grid slices still own:

- implicit tracks;
- dense and column-major auto-flow;
- intrinsic and fractional track sizing;
- item self-alignment;
- named lines, negative lines and placement shorthands;
- `repeat()`, `minmax()`, subgrid and broader writing-mode interactions.
