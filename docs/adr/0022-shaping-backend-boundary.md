# ADR-0022: Shaping backend boundary

## Status

Accepted.

## Context

R0 already segments text into grapheme-safe shaping runs with one font face and one bidi level. The remaining boundary must support a real OpenType shaper without leaking backend-specific glyph data into bidi, fallback, line breaking, fragments, or paint.

## Decision

Introduce `ShapingBackend::shape_run`, taking source text, one `ShapingRun`, and its resolved `FontFace`. The backend returns a `ShapedRun` containing glyph IDs, per-glyph advances and offsets, source-cluster ranges, aggregate advance, and font metrics. Keep the current aggregate `ShapedText` contract for line breaking until the line-layout layer is ready to consume backend glyph runs directly.

The bootstrap `FixedTextShaper` implements both contracts. Its backend implementation emits one deterministic glyph per grapheme cluster, uses the selected face metrics/advance, preserves logical source ranges, and reverses glyph order for RTL shaping runs while retaining source mapping.

## Consequences

A real OpenType shaping implementation can be introduced behind the new trait without changing bidi segmentation, font fallback, source identity, fragmentation, or retained paint. Script/language tags, OpenType feature selection, variation axes, glyph extents, vertical text, and platform font discovery remain future work.
