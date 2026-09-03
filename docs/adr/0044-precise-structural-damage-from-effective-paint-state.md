# ADR-0044 — Precise structural damage from effective paint state

Status: Accepted

## Context

ADR-0043 made damage replay local even when a display list contains clips, stacking-context markers, transforms or opacity scopes. `DamageRegion::between`, however, still treated the mere presence of any structural command as a reason to invalidate every effective paint bound in both display lists.

That rule was safe but defeated retained-paint locality whenever one transformed or clipped item changed beside otherwise stable content. Structural commands do not paint by themselves; their relevant effect is the device-space state they contribute to individual paint items.

## Decision

Compute damage from an indexed effective paint state keyed by `DisplayItemId`.

For each paint-producing command, the damage pass records:

- paint ordinal, so reordering overlapping items is observable;
- device-space bounds after the active transform stack and clip stack;
- effective solid color after the active opacity stack for fills/text placeholders;
- transformed destination, image revision reference and effective opacity for images.

Structural commands update transform, clip and opacity state while the list is scanned. Stacking-context markers currently preserve ordering semantics but do not independently alter raster output.

`DamageRegion::between` compares the effective item associated with each stable display-item ID. Only changed, inserted, removed or reordered paint items contribute their previous and/or current effective bounds. A first render still damages every visible effective paint bound.

The previous blanket structural invalidation path and the now-unused structural-bound helpers are removed.

## Consequences

A transform, clip or opacity change can now invalidate only the paint items whose effective output changed. Stable paint outside the affected structural scope remains outside the damage region, while paint-order changes are still conservatively detected through the stored ordinal.

This keeps damage computation aligned with the device-space semantics used by partial replay and gives retained display-list identity a useful role across richer structural paint scopes without exposing renderer internals outside `rarog-paint`.
