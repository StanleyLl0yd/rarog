# ADR-0036: Mixed text and paint-only style updates

## Status

Accepted for R1 — Flame.

## Context

ADR-0035 established incremental reflow for ordinary `CharacterData` mutations while conservatively sending every update batch that also contained a style mutation to `FullRebuild`. Rarog already classifies computed-style differences into layout-affecting and paint-only changes, and both the paint-only retained path and text flow-reflow path are independently verified against fresh rendering.

Treating every mixed batch as structural uncertainty therefore discards safe reuse when the style difference changes only paint state such as background or color.

## Decision

A batch containing ordinary text mutation and style mutation may remain incremental when every computed style difference in the batch is paint-only according to the existing layout-style classification.

Style candidates are still fully recomputed before the text refresh. If any style difference changes layout geometry, display membership, formatting-context behavior, or otherwise crosses an existing incremental safety boundary, the whole batch uses `FullRebuild`.

For a safe mixed batch, Rarog refreshes the retained text node and intrinsic ancestor chain, applies the paint-only style updates to the retained Layout Tree, performs the existing flow-aware fragment relayout required by the text change, rebuilds paint from the resulting Fragment Tree, computes damage against the previous display list, and rasterizes only damaged framebuffer regions.

## Consequences

- A text edit accompanied by a paint-only style change no longer forces a full Layout Tree rebuild.
- Geometry-affecting mixed updates remain deliberately conservative.
- The decision reuses existing style classification rather than introducing a second invalidation taxonomy.
- Correctness remains defined by equality with a fresh full render for the same final DOM and style state.

## Invariants

1. Mixed updates are incremental only after computed-style classification proves every style change non-layout-affecting.
2. Display/formatting-context changes always use the existing full-rebuild boundary.
3. Text refresh still preserves retained LayoutNode identity.
4. Damage is derived after both text and paint-only style changes are represented in the updated render state.
5. Any uncertainty falls back to deterministic full rebuild.
