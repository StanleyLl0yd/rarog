# ADR-0016: Text shaping boundary

## Status

Accepted.

## Context

Line boxes and text ranges need measurement data, but layout must not depend directly on a specific font or shaping library.

## Decision

Introduce `TextShaper`, `ShapedText`, `GlyphCluster`, and `FontMetrics` in the layout-facing text model. The R0 bootstrap shaper is deterministic and fixed-advance. Line breaking consumes shaped cluster advances and source ranges.

No external font or shaping backend is selected by this decision.

## Consequences

A real shaping implementation can later provide variable advances, multi-character clusters, font metrics, bidi-aware ordering, and font fallback behind the same boundary. Layout and retained-paint identity remain independent of the concrete backend.
