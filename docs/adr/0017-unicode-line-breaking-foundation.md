# ADR-0017: Unicode line-breaking foundation

## Status

Accepted.

## Context

The initial line breaker consumed shaped advances but could only split text by width. It had no representation of legal or mandatory text boundaries.

## Decision

Introduce explicit `BreakOpportunity` values and a `UnicodeLineBreaker`. R0 recognizes mandatory line separators, breakable Unicode whitespace, hyphen opportunities, non-breaking spaces, and basic CJK ideographic boundaries. Mandatory separators receive zero advance from the bootstrap shaper.

This is a deterministic UAX #14-oriented subset. It is not full UAX #14 conformance and does not yet implement language tailoring, grapheme-boundary protection, CSS `line-break`, `word-break`, `overflow-wrap`, or hyphenation.

## Consequences

Line layout now separates shaping widths from break policy. A standards-complete Unicode line-break implementation can replace the bootstrap classifier without changing `TextRange`, `GlyphCluster`, line-box, fragment, or retained-paint identity contracts.
