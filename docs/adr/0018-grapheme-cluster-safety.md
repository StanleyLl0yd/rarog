# ADR-0018: Grapheme-cluster safety

## Status

Accepted.

## Context

Unicode-aware line breaking introduced legal and mandatory break opportunities, but the bootstrap shaper still emitted one cluster per scalar value. Emergency wrapping could therefore split combining sequences or emoji sequences internally.

## Decision

Keep `TextRange` indexed by Unicode scalar position, but allow each `GlyphCluster` to span multiple scalar values. Introduce deterministic grapheme-safe boundaries before shaping and require line-break opportunities and emergency breaks to land on those boundaries.

The R0 classifier preserves combining marks, variation selectors, emoji modifiers, CRLF, regional-indicator pairs, and basic emoji ZWJ sequences. It is intentionally a UAX #29-oriented subset, not full Unicode grapheme segmentation conformance.

## Consequences

Shaping, line breaking, fragmentation, and retained paint can now treat clusters as indivisible without changing existing `TextRange` or fragment identity contracts. A standards-complete grapheme segmenter can replace the bootstrap classifier later.
