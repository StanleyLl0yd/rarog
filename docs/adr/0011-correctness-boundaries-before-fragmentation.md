# ADR-0011: Correctness boundaries before fragmentation

## Status

Accepted.

## Context

R0 is about to add clip/stacking semantics and then allow multiple fragments per layout source. Three bootstrap assumptions would become correctness hazards at that boundary: non-finite CSS lengths could reach geometry, public render construction could panic on hostile viewport sizes, and display-item identity based only on source plus paint slot could collide when one source creates multiple fragments. `RenderSession` also owns a mutation-history checkpoint that must not be advanced by an external caller.

## Decision

1. Bootstrap CSS length parsing rejects all non-finite `f32` values.
2. `render_html`, `render_html_against` and `RenderSession::new` are fallible and propagate framebuffer validation through `RenderError`.
3. `DisplayItemId` contains source identity, Fragment identity and paint slot. Generated display lists assert ID uniqueness and damage comparison fails loudly on duplicate IDs.
4. `RenderSession` exposes a mutation-only `DocumentEditor` instead of a raw mutable `Document`, keeping mutation-journal pruning engine-owned.
5. A deterministic mutation-stress test exercises DOM invariants over a long reproducible sequence without adding a fuzzing dependency to the runtime workspace.

## Consequences

The public R0 render API now requires error handling. Deterministic display-list/signature goldens change because display identity has a richer representation, while framebuffer output remains unchanged. Fragment IDs are still snapshot identities; a later fragmentation/retained-paint ADR must define stable fragment ordinals if retained identity needs to survive fragment reconstruction.
