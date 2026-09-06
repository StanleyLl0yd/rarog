# ADR-0090: Separate non-spanning Grid intrinsic base and growth state

## Status

Accepted.

## Context

ADR-0085 introduced explicit track base sizes and growth limits, and ADR-0089 preserved distinct semantic item contributions through the fragment builder.

The remaining bounded single-span resolver still selected one contribution kind and used that scalar only to grow the track base size. That preserves the historical #182 max-content geometry, but it cannot represent the intrinsic track-sizing distinction between a track's base-size target and its growth-limit target.

For the currently supported fixed/auto explicit tracks and non-spanning intrinsic items, Rarog can model that distinction without enabling new CSS syntax or changing current rendering.

## Decision

Rarog adds a layout-internal non-spanning intrinsic state resolver with independently selected contribution kinds for:

- base-size growth;
- growth-limit resolution.

The base-size phase continues to use the span-ordered planned-increase mechanism from ADR-0086/ADR-0088. This keeps one order-independent base growth path instead of introducing a second special-case algorithm.

For each supported single-track `auto` item, the resolver also records the selected growth contribution. After the base-size round completes:

- an `auto` track's finite growth limit becomes the largest selected growth target for that track;
- the growth limit is never allowed below the resolved base size;
- an unused `auto` track whose growth limit remains conceptually infinite is closed to its base size;
- fixed tracks retain their fixed base size and matching finite growth limit.

The existing semantic geometry resolver remains compatibility-preserving in this slice: it passes the same selected contribution kind for both base and growth. Current production Grid layout selects `MaxContent`, so CSS-visible geometry remains unchanged.

Primitive tests additionally exercise the future standards-oriented pair `Minimum` for the base size and `MaxContent` for the growth limit.

## Consequences

Rarog now has explicit state capable of representing the bounded non-spanning intrinsic base/growth distinction required before Maximize Tracks and auto-track stretch can be introduced.

The span-round mechanism remains production-used.

No public API or CSS-visible geometry changes in this slice.

## Deferred

This ADR does not:

- switch production geometry from the #182 max-content compatibility behavior to minimum-based base sizes;
- distribute definite free space during Maximize Tracks;
- stretch `auto` tracks after flexible sizing;
- resolve flexible tracks;
- implement richer intrinsic max track functions;
- enable intrinsic spanning items in CSS layout.

Those phases must be composed before the compatibility projection can be removed without causing an incomplete intermediate rendering model.
