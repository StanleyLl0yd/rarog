# ADR-0019: Bidirectional text foundation

## Status

Accepted.

## Context

Grapheme-safe shaping and Unicode-aware line breaking still treated text as a single logical direction. A browser text pipeline needs an explicit boundary between logical source ranges and visual ordering before a real shaping backend can support RTL scripts correctly.

## Decision

Introduce `TextDirection`, `BidiLevel`, and `BidiRun` in the layout text boundary. Keep all ranges indexed by Unicode scalar position. Determine the bootstrap paragraph direction from the first strong character, group deterministic LTR/RTL runs, and expose a level-based visual run ordering helper.

The R0 classifier recognizes Hebrew and Arabic-family ranges as RTL and alphabetic/digit text as LTR. Neutral characters inherit the preceding strong direction or paragraph base. This is intentionally a UAX #9-oriented subset rather than full conformance.

## Consequences

Logical source identity stays stable while later shaping and painting stages can consume explicit bidi runs. Full embedding controls, isolates, weak/neutral resolution, mirroring, bracket pairing, and standards-complete level resolution remain future work.
