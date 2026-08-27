# ADR-0020: Font fallback foundation

## Status

Accepted.

## Context

The text pipeline has grapheme-safe segmentation and explicit bidi runs, but shaping still assumes one synthetic font for every character. A real browser must select different font faces for unsupported scripts without changing logical source identity or splitting grapheme clusters.

## Decision

Introduce explicit font-face identity, families, coverage classes, a deterministic fallback chain, and scalar-indexed `FontRun` values. Select fallback per grapheme cluster and coalesce adjacent clusters using the same face. The bootstrap chain contains Latin/Cyrillic, Hebrew/Arabic, CJK, emoji, and mandatory LastResort faces.

`TextRun` exposes the selected font runs while the existing bootstrap shaper remains unchanged. A later shaping backend may shape each `(bidi run × font run)` segment independently.

## Consequences

Font fallback can evolve independently of DOM, bidi, line breaking, fragmentation, and retained paint. The R0 faces are deterministic architecture placeholders, not real font files or platform font discovery. Script/language-sensitive fallback, variable fonts, OpenType features, platform enumeration, and glyph-level fallback remain future work.
