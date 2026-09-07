# ADR-0094: Grid track-group content positioning and distribution

## Status

Accepted.

## Context

ADR-0093 wires the supported non-spanning `auto` track subset into definite intrinsic sizing, but intentionally keeps a max-content compatibility path for `center`, `end` and distributed `space-*` content alignment because the resulting Grid track group was still always laid out from the start edge with only the declared gap.

CSS Box Alignment treats Grid tracks as the alignment subjects. Positional content alignment moves the resolved track group inside the Grid container, while distributed alignment inserts extra space between tracks in addition to the declared gutter.

Without a separate track-group distribution step, applying the intrinsic sizing phases for these values would produce correct track sizes but incorrect placement.

## Decision

Rarog adds a layout-internal `GridTrackGroupAlignment` and a resolved `GridTrackGroupDistribution { offset, gap }`.

The resolver operates after track sizing and accepts:

- resolved track sizes;
- the declared fixed gap;
- definite or indefinite available space;
- the axis;
- a bounded content-alignment mode.

For definite available space:

- `start` uses zero offset and the declared gap;
- `end` offsets the track group by all remaining free space;
- `center` offsets by half the remaining free space;
- `space-between` distributes positive free space equally between adjacent tracks;
- `space-around` distributes equal slots around each track, producing half a slot at each outer edge;
- `space-evenly` produces equal slots between tracks and both outer edges.

Distributed spacing is added to the declared gap rather than replacing it.

For negative free space, the currently supported distributed values use their safe fallback:

- `space-between` falls back to start;
- `space-around` and `space-evenly` fall back to safe center, which in the bounded no-scroll-safety model is represented by a start-safe zero offset.

Explicit `end` and `center` remain positional alignment and may therefore produce negative offsets when the track group overflows.

A one-track `space-between` falls back to start.

For indefinite available space, no additional distribution occurs and the declared gap is preserved.

## Integration

The FragmentBuilder now sizes all currently parsed Grid content-alignment values through the same semantic Minimum -> MaxContent intrinsic pipeline when the corresponding axis has definite space.

After sizing:

- `normal`, `stretch` and `flex-start` map to start positioning;
- `flex-end` maps to end;
- `center` maps to center;
- `space-between`, `space-around` and `space-evenly` use the matching distribution mode.

The resolved column distribution is applied to the provisional geometry used for auto-row natural-height measurement, so item measurement sees the same effective column gaps/offsets as final layout.

The resolved row distribution is applied after row sizing.

The final `layout_fixed_grid` call receives the distributed origin and effective gaps.

## Consequences

The separate max-content compatibility path for currently parsed Grid content-position/distribution values is no longer required.

Declared Grid gaps compose with distributed alignment.

Track sizing remains independent from track-group positioning.

Content alignment is layout-owned and does not leak CSS parser AST types into Grid geometry primitives.

## Deferred

This ADR does not add:

- safe/unsafe overflow-position syntax;
- left/right logical-position variants;
- baseline content alignment;
- writing-mode-sensitive start/end mapping beyond the current horizontal physical subset;
- `place-content`;
- flexible or implicit Grid tracks;
- CSS-visible intrinsic multi-track spanning.

Those remain separate bounded standards slices.
