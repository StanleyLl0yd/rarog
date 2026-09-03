# ADR-0043 — Partial damage replay through structural paint scopes

Status: Accepted

## Context

Rarog's retained display list and damage tracking already identify changed paint bounds, but R0 rasterization deliberately fell back to clearing and replaying the entire framebuffer whenever a display list contained structural commands such as clips, stacking-context markers, transforms or opacity scopes.

That fallback preserved correctness but defeated damage locality precisely for richer retained paint scenarios. The full rasterizer already maintains all structural scope state needed to replay a list correctly; what was missing was a device-space damage clip that survives while those scopes are evaluated.

## Decision

Share the structural display-list interpreter between full rasterization and damage rasterization.

The framebuffer rasterizer now:

- keeps full rasterization as a wrapper that supplies the entire framebuffer as the initial clip;
- provides an internal clipped replay path whose initial clip is the intersection of the framebuffer and a device-space damage rectangle;
- evaluates the complete display list for each damage rectangle so transform, clip, stacking and opacity stacks are reconstructed exactly as in a full replay;
- paints only where transformed paint bounds intersect the active structural clips and the initial damage clip;
- clears only the clipped damage rectangle before replaying it;
- applies the same path to decoded images as to fills/text placeholders.

The old `is_structural` full-frame fallback is removed from damage rasterization.

## Consequences

Damage rasterization can now remain partial even when the display list contains nested clips, transforms, opacity scopes or stacking-context markers. Pixels outside the supplied damage rectangle remain untouched, while incremental replay inside damage is required to match a fresh full raster.

`DamageRegion::between` remains conservative for structural display lists: it may still include all effective paint bounds when any structural command is present. Narrowing structural damage computation is a separate optimization and does not need to change this replay correctness boundary.
