# ADR-0039: Append-only structural subtree reflow

## Status

Accepted for R1 — Flame.

## Context

Rarog can retain Layout Nodes across text mutations and can reuse unchanged stylesheet sources across ordinary structural rebuilds. Connected `ChildAdded` mutations still rebuilt the complete Layout Tree even when the mutation was confined to an existing rendered parent and did not change stylesheet sources.

## Decision

For a connected `ChildAdded` whose parent already maps to retained layout and whose inserted subtree does not change stylesheet sources, rebuild only the parent's retained Layout subtree from the final DOM and current `StyleSet`.

Existing DOM nodes that survive the rebuild retain their `LayoutNodeId`. New rendered DOM nodes receive IDs above the current retained maximum. Intrinsic sizes are recomputed through retained ancestors.

The refreshed parent is routed through the existing flow-aware fragment relayout path, which in turn uses retained display-list replacement and damage-scoped rasterization.

Style candidates inside the refreshed subtree are consumed by the subtree rebuild; invalidation candidates outside the subtree still use the normal incremental style classification.

`Reparented`, stylesheet-source changes, unmappable parents, and any subtree rebuild that cannot preserve the required layout boundary remain conservative full-rebuild fallbacks.

## Consequences

- Ordinary connected child appends can avoid rebuilding the complete Layout Tree.
- Layout identity survives structural growth for pre-existing DOM nodes.
- Structural selectors within the rebuilt subtree are recomputed from the final DOM.
- The first structural slice does not yet support reparenting or arbitrary detach/removal.

## Invariants

1. Existing surviving DOM-to-layout identities are preserved.
2. New layout IDs do not collide with retained IDs.
3. Intrinsic sizes are recomputed through every retained ancestor affected by the refreshed subtree.
4. Incremental framebuffer output matches a fresh full render.
5. Retained display-list replacement must remain observable and validated rather than silently degrading.
6. Any unsupported structural or stylesheet-source case falls back safely.
