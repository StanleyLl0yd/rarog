# ADR-0037: Retained display-list suffixes for flow relayout

## Status

Accepted for R1 — Flame.

## Context

Rarog already retains Layout and Fragment state across incremental geometry changes and computes damage against stable display-item IDs. `FlowRelayout` rebuilds only the affected root-flow suffix and reuses matching Fragment IDs, but the engine still rebuilt the entire DisplayList afterwards. That preserved raster damage locality but not paint-list construction locality.

## Decision

When flow relayout can identify the earliest affected root-flow child, the engine snapshots only the previous Fragment suffix beginning at that child. After fragment relayout it derives display commands for the previous and current suffix and asks the existing retained display-list replacement mechanism to replace that exact contiguous range.

The replacement is accepted only when the previous command IDs and commands match exactly, both replacement lists are structurally balanced, the structural scope stack is identical on both sides of the range, and the candidate list retains unique IDs and balanced structure.

If any condition fails, Rarog rebuilds the complete DisplayList. Damage computation and damage-scoped rasterization then proceed exactly as before.

`IncrementalReport::retained_display_list` reports whether the update actually reused the existing DisplayList. Flow-relayout tests require this flag so retained paint cannot silently regress to a full list rebuild while preserving pixel correctness.

## Consequences

- Successful `FlowRelayout` now retains unaffected display-list prefixes as well as Fragment prefixes.
- Paint-list work is localized to the same root-flow suffix as fragment relayout.
- Exact structural/range checks preserve conservative fallback behavior around future stacking/clip/transform/opacity scopes.
- The engine still rebuilds the complete DisplayList when retained replacement cannot be proven safe.
- This is not yet retained paint for arbitrary structural DOM mutation or generalized stacking-context reconstruction.

## Invariants

1. A retained range must match the previous display commands exactly.
2. Replacement must not cross a structural-scope boundary.
3. Candidate display-item IDs remain unique.
4. Any ambiguous range or structural mismatch falls back to whole-list rebuild.
5. `retained_display_list` reflects the actual path taken, not the intended optimization.
6. Incremental framebuffer output must continue to match a fresh full render.
