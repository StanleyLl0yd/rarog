# ADR-0083: Intrinsic sizing for non-stretch Grid items

## Status

Accepted.

## Context

ADR-0079 introduced bounded Grid item self-alignment but intentionally required an explicit used size on every non-stretch axis. A Grid item with `justify-self` or `align-self` resolving away from stretch and an automatic corresponding size therefore failed closed.

ADR-0082 added retained intrinsic contributions and isolated block-size measurement for bounded `auto` track sizing. Those layout-owned measurements are now sufficient to remove the temporary item-sizing restriction without introducing CSS parser ownership into layout or a second content-measurement model.

## Decision

For the current horizontal writing-mode Grid slice, automatic Grid item sizing resolves as follows:

- an automatic inline size with effective stretch continues to fill the Grid area after physical margins, padding and borders;
- an automatic inline size without effective stretch uses the item's retained max-content intrinsic size;
- an explicit inline size keeps the existing explicit-size path;
- an automatic block size with effective stretch continues to fill the Grid area after physical margins, padding and borders;
- an automatic block size without effective stretch is measured through the existing isolated fragment builder at the item's resolved border-box inline size;
- an explicit block size keeps the existing explicit-size path;
- existing min/max constraints remain part of used-size resolution;
- self-alignment positions the resulting physical margin box inside the already-resolved Grid area;
- negative remaining alignment space is preserved rather than clamped;
- an empty non-stretch automatic item has a zero intrinsic border-box size and is aligned normally instead of causing the Grid item list to fail closed.

When an item participates in an `auto` row, its natural block contribution is measured at the same effective inline width that the item would use: a stretched automatic inline size uses the Grid area width, while non-stretch/intrinsic sizing uses the item's max-content inline size.

No new CSS syntax or parser-owned layout type is introduced.

## Consequences

Common Grid content can use `justify-self` / `align-self` with automatic item sizes instead of disappearing through the previous bounded fallback.

Track sizing and item sizing now reuse the same retained intrinsic data and isolated measurement boundary.

Default stretch behavior from the earlier Grid slices is unchanged.

ADR-0079's temporary fail-closed rule for non-stretch automatic item sizes is superseded by this decision; its other self-alignment constraints remain in force.

## Deferred

Later Grid slices still own:

- `justify-items`;
- baseline alignment;
- auto margins;
- intrinsic contribution distribution across spanning tracks;
- fractional tracks and full flexible track sizing;
- `minmax()`, min-content/max-content track functions and `repeat()`;
- implicit tracks and broader auto-flow;
- named lines/areas, negative lines and placement shorthands;
- writing-mode expansion and subgrid.
