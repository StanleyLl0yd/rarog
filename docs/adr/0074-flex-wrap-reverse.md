# ADR-0074: Flex wrap-reverse cross-axis direction

## Status

Accepted.

## Context

The wrapped Flexbox path now supports line collection, line gaps, line alignment, measured auto-height items and per-line stretch. The remaining cross-axis directional primitive is `flex-wrap: wrap-reverse`.

A naive implementation that mirrors completed border boxes would also invert the physical meaning of `margin-top` and `margin-bottom`, which is incorrect. The reversal must instead operate on the logical cross axis used to place lines and items.

## Decision

Rarog extends computed `flex-wrap` with `wrap-reverse`.

The layout-owned `FlexRowOptions` gains a cross-reverse flag with a default of false. Existing callers and APIs retain forward cross-axis behavior.

Wrapped line collection and source order are unchanged. Cross reversal affects only physical placement:

- logical cross-start maps to the physical cross-end;
- source-order line 0 is placed nearest the physical cross-end;
- subsequent lines advance toward the physical cross-start;
- `align-content:flex-start` aligns toward physical cross-end;
- `align-content:flex-end` aligns toward physical cross-start;
- center and distributed values retain their symmetric spacing semantics.

Inside each flex line, `FlexCrossAlignment::Start` and `End` swap their physical meaning. `Center` and `Stretch` are unchanged.

Physical margins are not swapped. Alignment continues to operate on the physical margin box, so asymmetric top/bottom margins keep their declared sides.

Measured wrapped auto-height items and per-line stretch reuse the same cross-reverse line geometry. Fragment/source order remains DOM/source order.

Retained style updates already treat `flex-wrap` changes as flex-container geometry changes, so transitions to or from `wrap-reverse` reuse the existing parent-container flow relayout path.

## Consequences

Wrapped Flexbox now supports both forward and reversed cross-axis line stacking without duplicating line collection, grow/shrink, line alignment or auto-height measurement.

The logical direction is represented in layout-owned options rather than leaking the CSS enum into geometry APIs.

## Deferred

Later R3 slices still own:

- `flex-direction: row-reverse`;
- writing-mode interactions;
- baseline alignment;
- auto margins;
- intrinsic/auto main-size flex bases and full freeze/redistribution.
