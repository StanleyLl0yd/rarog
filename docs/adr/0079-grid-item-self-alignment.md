# ADR-0079: Bounded Grid item self-alignment

## Status

Accepted.

## Context

The explicit fixed Grid path supports spans and bounded auto-placement, but every item currently uses the default stretch position inside its resolved Grid area.

Full Box Alignment depends on intrinsic sizing, writing modes, baseline groups, auto margins and container/item alignment interactions that are intentionally broader than this slice.

## Decision

Rarog adds computed `justify-self` with a bounded Copy enum:

- `auto`;
- `stretch` / `normal`;
- `start` / `flex-start`;
- `end` / `flex-end`;
- `center`.

The existing `align-self` parser also accepts `start` and `end` aliases in addition to the already-supported bounded values.

For Grid only, item alignment resolves as follows:

- `justify-self:auto` uses the current Grid initial inline-axis behavior of stretch;
- `align-self:auto` resolves through the Grid container's existing `align-items`;
- stretch preserves the existing auto-size-to-area behavior;
- start/end/center align the item's physical margin box within the resolved Grid area;
- physical margins are never swapped.

A non-stretch item must have an explicit used size on the corresponding axis in this slice. Non-stretch `width:auto` or `height:auto` fails closed because intrinsic Grid item sizing is not yet implemented.

Explicit sizes remain subject to existing min/max clamps and may overflow the Grid area. Negative remaining alignment space is preserved rather than silently clamped.

Grid container `align-items` changes and Grid item `align-self` / `justify-self` changes are retained-layout invalidations of the Grid formatting root.

## Consequences

Common fixed-size Grid card/icon alignment works without broadening track sizing.

The default stretch behavior from earlier Grid slices is unchanged.

Grid `align-items` now has a bounded block-axis effect when items use `align-self:auto`.

## Deferred

Later R3 slices still own:

- `justify-items`;
- intrinsic auto-size for non-stretch items;
- baseline alignment;
- auto margins;
- writing-mode expansion;
- intrinsic and fractional track sizing;
- implicit tracks, named lines, `repeat()`, `minmax()` and subgrid.
