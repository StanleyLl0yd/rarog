# ADR-0066: Flex main-axis alignment boundary

## Status

Accepted.

## Context

ADR-0065 added bounded grow/shrink sizing for the first single-line flex row. Once flexible sizes are resolved, remaining main-axis free space still needs standards-inspired placement before Rarog can claim a useful first alignment slice.

This change must not make the existing fixed-row and flexible-row APIs source-incompatible, and it must not mix main-axis alignment with cross-axis alignment, wrapping, reverse directions, gaps or auto margins.

## Decision

Rarog adds a layout-owned `FlexMainAlignment` value and explicit alignment variants of the existing row entry points. The existing public entry points remain unchanged and delegate to `FlexMainAlignment::Start`.

The CSS layer exposes a bounded computed `justify-content` value with these supported values:

- `flex-start`
- `flex-end`
- `center`
- `space-between`
- `space-around`
- `space-evenly`

For this first flex slice, CSS `normal` maps to the same used behavior as `flex-start`.

Main-axis alignment runs after grow/shrink sizing. Therefore any free space intentionally left undistributed by grow factors whose sum is below one remains available to `justify-content`.

For non-negative remaining free space:

- `flex-start` leaves all free space after the items;
- `flex-end` places all free space before the items;
- `center` splits it before and after the row;
- `space-between` distributes it only between items;
- `space-around` gives every item equal surrounding shares;
- `space-evenly` produces equal leading, inter-item and trailing spacing.

For negative remaining free space, positional `flex-end` and `center` preserve their overflow alignment. Distributed values use the safe start fallback rather than creating additional start-side overflow.

The row's reported `content_size.width` remains the measured outer item span before alignment spacing. Alignment changes item placement inside the available main size; it does not redefine the intrinsic measured span.

A retained flex container is relaid out when its computed `justify-content` changes.

## Consequences

Rarog now has a real first main-axis Box Alignment path over the same engine-owned flex geometry used by grow/shrink. Existing callers of the pre-alignment row APIs keep their prior start-aligned behavior.

The bounded computed representation intentionally does not yet expose the broader Box Alignment grammar such as `start`, `end`, `left`, `right`, or explicit `safe`/`unsafe` overflow modifiers.

## Deferred

Later R3 slices still own:

- `align-items`, `align-self` and flex baseline alignment;
- auto margins;
- `gap`;
- wrapping and multi-line alignment;
- reverse directions and writing modes;
- broader Box Alignment keywords;
- intrinsic/auto flex bases and the full main-size freeze/redistribution algorithm.
