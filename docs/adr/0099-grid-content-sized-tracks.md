# ADR-0099: Bounded min-content and max-content Grid tracks

## Status

Accepted.

## Context

R3 now has fixed, `auto` and definite-space fractional track sizing. The remaining Grid closure item calls for a bounded richer maximum-track slice without coupling CSS parser internals to layout or approximating unsupported phases.

Bare `min-content` and `max-content` track breadths are a useful next boundary because their single-span intrinsic behavior can be expressed directly with the semantic contribution classes already owned by layout.

Multi-track items crossing these track types require additional intrinsic distribution rules and remain a separate problem.

## Decision

The CSS computed track model adds `MinContent` and `MaxContent` variants and the bounded track-list parser accepts ASCII-case-insensitive `min-content` / `max-content` keywords.

Layout adds corresponding `GridTrackSizing` variants.

For a single-span item:

- a `min-content` track uses the item's min-content contribution for both its base size and growth limit;
- a `max-content` track uses the item's max-content contribution for both its base size and growth limit;
- an empty content-sized track closes its unused infinite growth limit to its zero base size.

These track types are intrinsic but are not stretchable `auto` tracks.

A multi-track intrinsic item crossing either content-sized track fails closed with `UnsupportedIntrinsicSpan`. Existing spanning support remains limited to the validated fixed/`auto` subset.

## Validation

Layout regressions verify track-specific contribution selection and the spanning failure boundary.

A CSS-visible regression lays out two `hello world` items in `min-content max-content` columns and verifies the retained text measurements produce 40px and 88px tracks.

Parser coverage verifies computed metadata accepts the new keywords without exposing parser AST ownership to layout.

## Consequences

R3 gains a concrete richer max-track slice while preserving explicit failure boundaries around unsupported intrinsic spanning.

This is sufficient for the bounded R3 Grid closure criterion. Full CSS Grid grammar and the remaining advanced track sizing algorithms are not prerequisites for R3 completion.

## Deferred

Deferred work includes `minmax()`, fit-content, percentage tracks, indefinite flex fractions, flexible/content-sized intrinsic spanning, implicit tracks, repeat syntax, named lines and subgrid.
