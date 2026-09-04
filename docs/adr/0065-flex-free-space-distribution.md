# ADR-0065: Flex free-space sizing boundary

## Status

Accepted.

## Context

ADR-0064 connected `display: flex` to the bounded single-line row primitive while deliberately retaining fixed resolved item sizes. The next R3 slice needs real main-axis flexible sizing without making the existing public fixed-row input source-incompatible or pretending that intrinsic flex bases and the complete min/max freeze algorithm already exist.

## Decision

Rarog adds `flex-grow` and `flex-shrink` as non-inherited computed-style numbers with CSS initial values of `0` and `1`.

The fixed `FlexRowItem` and `layout_single_line_flex_row` API remain unchanged. Flexible sizing is represented by `FlexibleFlexRowItem` and `layout_flexible_single_line_flex_row`.

For the current single-line row slice:

- positive free space is distributed by grow factor;
- when the sum of grow factors is below one, only that fraction of initial free space is distributed;
- negative free space is distributed by scaled shrink factor, using `shrink * base width`;
- when the sum of shrink factors is below one, only that fraction of the initial deficit is removed;
- margins are outside the flexible base width and participate in available-space accounting;
- the resolved flexible border-box width is propagated into actual block/flex fragment geometry;
- padding and border form a non-shrinkable border-box floor in this slice;
- currently resolved `min-width` and `max-width` values are carried into the flexible geometry as main-size limits;
- a retained flex parent is relaid out when a direct item's grow or shrink factor changes;
- non-finite or negative factors and invalid size limits are rejected before entering layout.

If proportional sizing would cross a main-size limit and therefore require the later freeze/redistribution algorithm, this bounded slice fails closed. The same is true if unconstrained proportional shrink would require a negative border-box width. It does not clamp silently, violate padding/border geometry or emit invalid coordinates.

## Consequences

Rarog now has a real first flexible main-axis sizing path while preserving the earlier fixed-row API. Default `flex-shrink: 1` means overflowing fixed-base items may shrink in a flex container when the bounded algorithm can resolve them safely.

The algorithm starts from the currently resolved fixed width, including the existing pre-flex min/max clamp. It preserves those limits during flexible sizing by rejecting cases that would require freezing one item at a limit and redistributing the remainder. The standards freeze/redistribution algorithm itself is deliberately not claimed here.

## Deferred

The following remain later R3 work:

- auto and intrinsic flex base-size resolution;
- min/max main-size freezing and redistribution during flexing;
- intrinsic automatic minimum sizes;
- `flex-basis` and `flex` shorthand;
- main/cross-axis alignment and baseline alignment;
- `gap`;
- wrapping;
- reverse directions and writing modes;
- negative margins;
- anonymous non-whitespace flex items.
