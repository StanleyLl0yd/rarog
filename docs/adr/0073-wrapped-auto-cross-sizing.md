# ADR-0073: Measured auto-height items in wrapped flex lines

## Status

Accepted.

## Context

ADR-0071 introduced wrapping and ADR-0072 added flex-line `align-content`, but wrapped flex items still required an explicit height. Reusing the definite single-row auto-height implementation directly would be incorrect because a wrapped item's stretch target is the resolved cross size of its own flex line, not the container cross size.

Auto-height items may also contain real block or flex content whose natural height depends on the flexed main-axis width. Horizontal intrinsic metadata alone cannot provide that cross size.

## Decision

Rarog keeps the existing public `FlexRowItem`, `FlexibleFlexRowItem`, and wrapped layout entry point unchanged.

A new optional parallel `FlexCrossSizeMetadata` slice identifies auto cross-size items and carries bounded border-box min/max cross-size limits. The existing wrapped API calls the metadata-aware implementation with an empty slice.

Fragment construction uses a two-phase wrapped auto-height path:

1. build provisional wrapped geometry with zero/min-constrained auto content height to resolve line membership and flexed main-axis widths;
2. measure each auto-height item at its resolved border-box width using an isolated temporary `FragmentBuilder` that reuses the prepared margin profiles but has its own fragment-id allocator;
3. update the item's natural border-box height and run final wrapped layout;
4. after `align-content` resolves each line cross size, stretch only auto items whose effective `align-items` / `align-self` value is `stretch`.

Final stretch subtracts the item's margins from the line cross size and clamps the resulting border-box height to the metadata min/max limits.

Non-stretch auto-height items retain their measured natural height.

The final fragment content-height override is derived from the final placement border-box height, so measured and stretched geometry share one final fragment-construction path.

## Consequences

Wrapped flex items with real content can now participate in auto cross sizing and default stretch without using the whole container as their stretch target.

Temporary measurement does not consume ids from the retained fragment tree.

The wrapped path performs one provisional layout and bounded temporary item layouts before the final wrapped pass when auto-height items are present.

## Deferred

Later R3 slices still own:

- `wrap-reverse`;
- reverse main-axis directions and writing modes;
- baseline alignment;
- auto margins;
- intrinsic/auto main-size flex bases and full freeze/redistribution.
