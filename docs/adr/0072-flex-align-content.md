# ADR-0072: Wrapped flex-line align-content

## Status

Accepted.

## Context

ADR-0071 introduced bounded multi-line wrapping but deliberately rejected wrapped containers with a definite or constrained cross size. Correctly placing multiple flex lines in extra cross-axis space requires `align-content`.

The existing single-line path already owns item cross-axis alignment. A line-distribution implementation should reuse that path rather than introduce a second item-placement algorithm.

## Decision

Rarog adds computed `align-content` with:

- `stretch` as the initial value;
- `normal`, mapped to the current flex used behavior of `stretch`;
- `flex-start`;
- `flex-end`;
- `center`;
- `space-between`;
- `space-around`;
- `space-evenly`.

The layout layer owns a separate `FlexContentAlignment` enum so CSS parser representation does not leak into layout APIs.

Wrapped layout proceeds in two phases:

1. collect lines and run the existing single-line flexible-row algorithm to measure each line's natural cross size;
2. resolve the wrapped container used cross size from explicit/min/max constraints, distribute remaining space according to `align-content`, then run each line again at its resolved cross size.

For `stretch`, positive remaining cross space is divided equally among flex lines. The second line-layout pass then reuses the existing `align-items` / `align-self` cross placement against the stretched line size.

Fixed `row-gap` remains part of the natural inter-line spacing. Space-distribution values add extra spacing separately instead of replacing the fixed gap.

When the resolved cross size is smaller than natural wrapped content, start/stretch/space-* keep the first line at the start edge, while end and center use the same negative-overflow positioning convention as the existing main-axis alignment path.

Wrapped auto-height items remain unsupported in this slice. They must not use the container cross size as their stretch target because their correct target is the resolved cross size of their own flex line.

## Consequences

Definite-height and min/max-constrained wrapped flex containers are now representable without a silent approximation.

Single-line item alignment remains the only item cross-placement implementation.

The multi-line path performs a deterministic second line-layout pass, trading bounded extra work for simpler ownership and less duplicated sizing logic.

## Deferred

Later R3 slices still own:

- per-line auto-height flex-item stretch and intrinsic wrapped cross sizing;
- `wrap-reverse`;
- baseline alignment;
- auto margins;
- reverse main-axis directions and writing modes;
- intrinsic/auto flex bases and full main-size freeze/redistribution.
