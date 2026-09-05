# ADR-0071: Bounded multi-line flex wrapping

## Status

Accepted.

## Context

The current Flexbox path owns single-row sizing, main/cross alignment, fixed gaps, per-item alignment and definite-row auto-height stretch. The next useful standards slice is line wrapping.

A fully general multi-line flex formatting context also requires `align-content`, reverse line stacking, intrinsic auto cross-size items and later writing-mode work. Implementing wrapping without an explicit boundary would silently make definite-height multi-line containers incorrect.

## Decision

Rarog adds computed `flex-wrap` with:

- `nowrap` as the initial value;
- `wrap`.

`wrap-reverse` remains unsupported in this slice.

The layout layer adds a multi-line result and a wrapped flexible-row entry point. It forms lines in source order using each item's outer base main size and the fixed main-axis gap. An item wider than the available main size remains alone on its line rather than creating an empty line.

Each collected line reuses the existing single-line flexible-row algorithm. Therefore grow/shrink distribution, fixed main-axis gaps, `justify-content`, `align-items` and per-item `align-self` are resolved independently for each line.

The computed `row-gap` is the fixed cross-axis gap between adjacent lines.

This first wrapped slice accepts only an auto-height flex container without `min-height` or `max-height`. Wrapped containers with a definite or constrained cross size fail explicitly at the layout primitive because distributing or stretching flex lines in that space requires `align-content`.

Wrapped items continue to require explicit heights. General intrinsic auto-height wrapped items remain outside this slice.

The fragment layer preserves item source order while using the multi-line placements. Retained updates relayout flex containers when `flex-wrap` or `row-gap` changes.

## Consequences

Rarog can now produce genuine multi-line Flexbox geometry for a bounded but useful auto-height case. Flexible sizing and justification are not reimplemented for wrapping, reducing divergence between single- and multi-line behavior.

`row-gap` now has real geometry in wrapped flex containers while remaining inert for single-line layout.

## Deferred

Later R3 slices still own:

- `align-content`;
- definite/min/max-height multi-line containers;
- `wrap-reverse`;
- intrinsic auto-height wrapped items;
- baseline alignment;
- auto margins;
- reverse main-axis directions and writing modes;
- intrinsic/auto flex bases and full main-size freeze/redistribution.
