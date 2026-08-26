# ADR-0009: Persistent dirty state and conservative incremental reuse

## Status

Accepted for R0 — Ember.

## Context

R0 already has generation-ordered DOM mutation records, conservative style/layout/paint invalidation flags, deterministic Layout/Fragment snapshots, stable display-item IDs and damage comparison. Until this decision, every render rebuilt all derived layout and fragment state, so the invalidation primitives were not yet connected to persistent frame state.

The first incremental step must prove reuse without making correctness depend on incomplete invalidation rules or on bootstrap parser/CSS behavior.

## Decision

`rarog-engine` owns a stateful R0 `RenderSession` and persistent `DirtyState`.

`DirtyState` accumulates invalidation entries from DOM generations until an update consumes them. The DOM remains unaware of CSS, layout or paint types.

The first incremental path is deliberately limited to mutations of `id`, `class` and inline `style` whose newly computed style does not change geometry or visibility. If only paint values change, Rarog patches the computed style stored on the existing Layout Tree and Fragment Tree nodes, preserving their geometry and identities.

Any structural mutation, character-data mutation, missing layout source, `display` change or geometry-affecting computed-style difference triggers a deterministic full rebuild.

The initial experiment still rebuilds the display list and rerasterizes the framebuffer. Retained display-list updates and damage-scoped rasterization are separate later steps.

## Consequences

- R0 now has a real state-reuse path rather than only invalidation data structures.
- Incremental behavior is an optimization; the full rebuild remains the correctness fallback.
- Layout/Fragment reuse can be tested independently from future retained-paint work.
- The experiment makes no performance claim because display-list generation and raster are still full-frame operations.
- Geometry-affecting incremental relayout remains future work.
- Standards-complete selector dependency invalidation remains future work.

## Invariants

1. DOM mutation records never contain layout or paint objects.
2. Dirty state is consumed through document generations, not hidden cross-crate callbacks.
3. A reuse path may run only when the engine can prove that layout geometry and visibility are unchanged.
4. When that proof is unavailable, Rarog rebuilds derived state.
5. Incremental and fallback paths must preserve deterministic correctness tests.
