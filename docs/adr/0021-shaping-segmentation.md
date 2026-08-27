# ADR-0021: Shaping segmentation

## Status

Accepted.

## Context

R0 already has grapheme-safe source ranges, logical bidi runs, and deterministic font fallback runs. A real text shaper must receive text with one direction/embedding level and one selected font face; passing a whole mixed-direction or mixed-font `TextRun` would force shaping backends to duplicate segmentation policy.

## Decision

Introduce scalar-indexed `ShapingRun` values containing `TextRange`, `FontFaceId`, and `BidiLevel`. Build them as the ordered intersection of logical bidi runs and font fallback runs, preserve grapheme-safe boundaries, and coalesce adjacent intersections when face and level are identical. `TextRun` exposes the resulting shaping segments directly.

## Consequences

The future OpenType backend can shape one segment at a time and return glyph clusters without owning bidi or fallback policy. Script/language tags, OpenType feature selection, variation axes, vertical text, full UAX #9 resolution, and platform font discovery remain separate future layers.
