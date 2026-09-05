# ADR-0075: Horizontal flex row-reverse

## Status

Accepted.

## Context

ADR-0074 added logical cross-axis reversal for wrapped lines. The corresponding horizontal main-axis reversal is `flex-direction: row-reverse`.

Reversing source arrays would make fragment order diverge from DOM/source order and complicate retained identity. Mirroring completed border boxes would also swap the effective meaning of physical `margin-left` and `margin-right`.

The current Rarog flex formatting context is horizontal. Accepting `column` or `column-reverse` before a vertical main-axis sizing and placement path exists would be a silent compatibility claim.

## Decision

Rarog adds computed `flex-direction` with:

- `row` as the initial value;
- `row-reverse`.

`column` and `column-reverse` remain unsupported in this slice.

The layout-owned `FlexRowOptions` gains a main-reverse flag with a default of false.

Item and line collection remains in source order. For a reverse row:

- logical main-start maps to the physical right edge;
- each item's outer box is advanced in source order from right to left;
- the border box is positioned using the item's physical right margin at the physical main-end and its physical left margin at the other side;
- fixed main-axis gaps remain between adjacent outer boxes;
- grow/shrink sizing is unchanged.

`justify-content` continues to resolve logical leading and distributed free space with the existing algorithm. The resulting offsets are applied with a negative physical X sign for a reverse row. Therefore flex-start, flex-end, center and distributed values retain logical semantics without swapping CSS enums.

Wrapped line collection remains source ordered. Each line independently uses the same main-reverse row placement, so row-reverse composes with `wrap`, `wrap-reverse`, line alignment and measured auto-height items.

Retained flex-container style invalidation includes computed `flex-direction`.

## Consequences

Rarog supports both horizontal main-axis directions while preserving DOM/fragment order and physical margins.

Existing row callers retain forward behavior because main reversal is an opt-in layout option.

## Deferred

Later R3 slices still own:

- vertical `column` / `column-reverse` Flexbox;
- writing-mode interactions;
- baseline alignment;
- auto margins;
- intrinsic/auto main-size flex bases and full freeze/redistribution.
