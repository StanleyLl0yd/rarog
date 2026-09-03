# ADR-0040: Retained reparent and detach structural reflow

## Status

Accepted for R1 — Flame.

## Context

Append-only child insertion can rebuild one retained Layout subtree, but reparenting affects both the old and new structural roots. A per-root retained-ID snapshot cannot preserve the moved subtree's `LayoutNodeId` once it is removed from the old parent.

## Decision

For ordinary `Reparented` mutations with unchanged stylesheet sources, collect one global DOM-to-`LayoutNodeId` map before any structural refresh, minimize the connected old/new parent roots, and rebuild all affected roots with one monotonically increasing new-ID allocator.

Surviving and moved rendered DOM nodes reuse their previous `LayoutNodeId` from the global snapshot. Newly rendered nodes receive IDs above the retained maximum. Detach is handled as the one-connected-root form of the same operation.

All refreshed structural roots feed the existing root-flow relayout, retained display-list replacement, damage comparison and damaged-region rasterization.

Moves involving `<style>` sources remain full-rebuild boundaries. A subtree created detached and then attached in the same generation also remains a full-rebuild boundary in this slice.

## Consequences

- Ordinary reparenting can retain Layout identity across both affected parents.
- A moved rendered subtree keeps its previous Layout identity after changing parent.
- Detaching an existing subtree no longer requires rebuilding the complete Layout Tree.
- Structural selector state is recomputed from the final DOM for both affected roots.
- Newly-created detached-subtree attachment is intentionally left for a later slice.

## Invariants

1. The retained DOM-to-layout identity snapshot is captured before any structural root is rebuilt.
2. One allocator is shared across the whole structural refresh batch so new IDs cannot collide.
3. Structural roots are minimized before rebuild so a descendant root is not rebuilt independently when an ancestor rebuild already covers it.
4. Moved surviving rendered DOM nodes retain their previous `LayoutNodeId`.
5. Incremental framebuffer output matches a fresh full render.
6. Retained display-list replacement remains validated and observable.
7. Stylesheet-source changes and unsupported structural histories fall back safely.
