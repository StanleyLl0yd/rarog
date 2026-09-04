# ADR-0067: Flex gap boundary

## Status

Accepted.

## Context

ADR-0066 added main-axis alignment to the bounded single-line flex row. A fixed inter-item gap must participate in the same free-space accounting as margins and item sizes: flex grow/shrink must not consume space reserved for gaps, and `justify-content` must operate on the space that remains after both flexible sizing and fixed gaps.

The CSS representation should also be ready for later multi-line flex layout without pretending that row gaps already affect a single row.

## Decision

Rarog represents `row-gap` and `column-gap` as separate non-inherited computed lengths with a used initial value of zero.

The CSS boundary accepts:

- `row-gap`;
- `column-gap`;
- one- or two-value `gap` shorthand;
- `normal`, mapped to the current zero used length.

Only finite non-negative pixel lengths are accepted in the current CSS subset. Negative or non-finite values are rejected.

For the current horizontal single-row Flexbox slice, `column-gap` is the main-axis gap. `row-gap` is retained in computed style for the later wrapping slice but has no geometric effect while there is only one flex line.

The layout primitive adds `FlexRowOptions`, which carries main-axis alignment and the fixed main-axis gap. Existing row entry points remain source-compatible and delegate through default options.

Main-axis gap space is reserved before grow/shrink free-space distribution. The resolved row then places the fixed gap between adjacent items, and `justify-content` operates on any space remaining after items, margins and fixed gaps.

A retained flex container is relaid out when its computed `column-gap` changes. A `row-gap`-only change does not trigger geometric relayout until multi-line flex layout exists.

## Consequences

Single-row Flexbox can now combine fixed gaps with grow/shrink and main-axis alignment without double-counting or allowing flexible items to consume gap space.

The options object becomes the extension boundary for later row parameters, avoiding a new public function for every additional flex setting while retaining the existing convenience entry points.

## Deferred

Later R3 slices still own:

- multi-line use of `row-gap`;
- percentage and broader CSS length support;
- cross-axis alignment and baseline alignment;
- auto margins;
- wrapping and multi-line alignment;
- reverse directions and writing modes;
- intrinsic/auto flex bases and the full main-size freeze/redistribution algorithm.
