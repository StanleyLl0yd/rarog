# ADR-0068: Flex cross-axis alignment boundary

## Status

Accepted.

## Context

The first Flexbox slices now cover single-row dispatch, grow/shrink, main-axis alignment and fixed gaps. Cross-axis positioning must use the same Rarog-owned geometry without conflating three different sizing cases:

1. an auto-height flex container whose line cross size is measured from its items;
2. a container with a definite content height;
3. an auto-height container whose existing `min-height` / `max-height` constraints change the final used content height.

The current bounded flex-item slice still requires explicit item `width` and `height`. It therefore cannot honestly claim the size-changing part of `align-items: stretch`, and it has no flex-item baseline data yet.

## Decision

Rarog adds a bounded non-inherited computed `align-items` representation with:

- `stretch` (initial value);
- `normal`, mapped to the current flex used behavior of `stretch`;
- `flex-start`;
- `flex-end`;
- `center`.

`baseline` is not accepted by this slice. It remains deferred until the flex formatting context owns the baseline information required to implement it.

The layout layer adds `FlexCrossAlignment` and extends `FlexRowOptions` with cross-axis alignment plus optional definite/minimum/maximum cross-size inputs. Existing row APIs remain source-compatible and default to start placement.

For cross-axis positioning:

- with no definite or constrained container cross size, the line uses the maximum outer cross size of its items;
- with a definite container content height, alignment uses that used height;
- with auto height plus min/max constraints, the measured natural line size is clamped by the same max-then-min rule as the existing container content-height calculation;
- `flex-start` positions the item margin box at the line start;
- `flex-end` positions it at the line end;
- `center` centers the item margin box;
- `stretch` preserves start placement and the explicit item height in this slice.

Positional end/center alignment may produce start-side overflow when an item is larger than the resolved line cross size. The slice does not silently force a safe fallback.

A retained flex container is relaid out when its computed `align-items` changes.

## Consequences

Single-row flex containers can now position fixed-height items on both axes, including containers whose used height comes from `min-height` or `max-height`.

The `stretch` representation is intentionally future-ready but does not resize explicit-height items. Since auto-height flex items remain outside the currently accepted layout slice, no supported input is silently rendered with false stretch semantics.

## Deferred

Later R3 slices still own:

- auto cross-size flex items and actual stretch sizing;
- `align-self`;
- flex baseline alignment;
- auto margins;
- wrapping and multi-line cross-axis alignment;
- reverse directions and writing modes;
- intrinsic/auto flex bases and full main-size freeze/redistribution.
