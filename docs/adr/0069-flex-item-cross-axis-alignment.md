# ADR-0069: Per-item flex cross-axis alignment

## Status

Accepted.

## Context

ADR-0068 added container-level cross-axis positioning through `align-items`. Flex items also need an independent `align-self` override, but the existing public `FlexRowItem` and `FlexibleFlexRowItem` structs are already usable through struct literals and must not gain a required field as part of this bounded slice.

The current Flexbox boundary still requires explicit item width and height. Baseline alignment and size-changing stretch therefore remain outside the supported geometry.

## Decision

Rarog adds a bounded non-inherited computed `align-self` representation with:

- `auto` as the initial value;
- `stretch`;
- `normal`, mapped to the current flex used behavior of `stretch`;
- `flex-start`;
- `flex-end`;
- `center`.

`baseline` remains unsupported until the flex formatting context owns the baseline information needed to implement it.

The layout primitive keeps the existing public item structs unchanged. Per-item cross-axis overrides are supplied through a parallel slice of `Option<FlexCrossAlignment>` values:

- `None` means `align-self:auto` and resolves to the container alignment;
- `Some(...)` overrides the container alignment for that item;
- an empty slice means all items use the container alignment and preserves existing call behavior;
- a non-empty slice must match the item count exactly or layout fails explicitly.

The flexible sizing path carries the override slice through to the resolved fixed-size row without changing grow/shrink behavior.

A flex item's retained style change is treated as parent-row geometry-affecting when `align-self` changes, just like flex grow/shrink factors.

## Consequences

Per-item cross-axis positioning can now override `align-items` without breaking public item construction APIs or coupling CSS enums into the layout primitive.

Existing callers that do not provide per-item alignment data retain exactly the previous container-level behavior.

## Deferred

Later R3 slices still own:

- auto cross-size flex items and actual stretch sizing;
- flex baseline alignment;
- auto margins;
- wrapping and multi-line alignment;
- reverse directions and writing modes;
- intrinsic/auto flex bases and full main-size freeze/redistribution.
