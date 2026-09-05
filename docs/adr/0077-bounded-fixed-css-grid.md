# ADR-0077: Bounded fixed CSS Grid integration

## Status

Accepted.

## Context

#148 introduced a Rarog-owned fixed explicit Grid geometry primitive without exposing CSS parser structures to layout. The next step is to connect a useful subset of CSS Grid while preserving the lightweight `ComputedStyle: Copy` model and explicit fail-closed behavior.

A general `Vec`-backed parser track list in computed style would broaden ownership and lifetime concerns before Grid sizing semantics exist.

## Decision

Rarog adds a bounded computed-style track representation with at most eight explicit fixed tracks per axis. The representation is a Copy value containing an internal fixed-size array and length; layout sees only resolved numeric track sizes.

The CSS subset accepts:

- `display:grid`;
- `grid-template-columns` and `grid-template-rows` with one to eight fixed finite non-negative lengths;
- `grid-column-start` and `grid-row-start` as positive 1-based explicit lines or `auto`;
- `grid-column-end: span N` and `grid-row-end: span N`, with `auto` mapping to span 1;
- existing `row-gap`, `column-gap`, and `gap`.

Unsupported fractional/intrinsic tracks, `repeat()`, named/negative lines, implicit tracks, auto-placement, absolute end-line placement and placement shorthands are rejected rather than approximated.

A Grid container establishes a formatting-context boundary for margin collapsing.

Grid items in this slice require explicit row and column starts. Source order is preserved, including overlap. The #148 primitive computes each item area.

For an auto-sized item, its margin box stretches to the grid area: physical margins are subtracted first, then padding and borders, and the remaining content size is clamped by existing min/max constraints. Explicit width/height remain explicit and may overflow the area.

Nested Grid and Flex containers reuse their existing sized fragment builders.

Retained updates treat track/gap changes as Grid-container layout changes and start/span changes as Grid-item layout changes that relayout the parent Grid formatting root.

## Consequences

Real `display:grid` becomes available for deterministic explicit fixed layouts without introducing parser-owned dynamic track vectors into layout or computed style.

The bounded representation makes the current limitation visible and testable instead of silently accepting unsupported track syntax.

## Deferred

Later R3 Grid slices own row-major auto-placement, item self-alignment, implicit tracks, intrinsic/fractional sizing, named lines, shorthands, `repeat()`, `minmax()`, subgrid, and broader writing-mode interactions.
