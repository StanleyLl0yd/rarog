# ADR-0027: Transform, opacity and retained paint scopes

## Status

Accepted.

## Context

R0 already has explicit clip and stacking-context commands plus contiguous retained display-list replacement. The remaining paint boundary needs to represent transforms and opacity without coupling paint to a particular compositor, and retained replacement needs a stronger proof when fragment ranges live inside nested structural scopes.

## Decision

The display list gains balanced transform and opacity scopes alongside clip and stacking scopes. `Transform2D` carries a backend-neutral affine transform. `Opacity` is finite and clamped to `[0, 1]`. The bootstrap software rasterizer applies nested transforms to rectangular paint geometry, evaluates clips in transformed device space, multiplies nested opacity and uses deterministic source-over blending. This is a representation and correctness foundation, not CSS transforms/compositing conformance.

Retained replacement now requires more than matching IDs. The live contiguous range must still contain the exact previous commands, and the structural scope stack at the beginning and end of that range must be identical. The replacement and resulting display lists must remain balanced and retain globally unique IDs. Fragment ordinals remain part of display-item identity, so separate fragments from one source node can be patched independently.

Structural damage computes conservative effective paint bounds through transform and clip scopes. Damage-scoped rasterization deliberately keeps the conservative full-frame fallback whenever any structural scope is present; partial replay through stacking/transform/opacity scopes is deferred to compositor work.

## Consequences

- Transform and opacity no longer require another display-list contract redesign.
- Retained patches fail closed if the previous slice is stale or crosses a structural-scope boundary.
- Fragmentation, stacking and clipping can coexist with retained range replacement in the R0 proof model.
- The software path remains deterministic while later GPU/compositor backends may consume the same commands differently.
- CSS transform-origin, 3D transforms, stacking-order calculation, isolated opacity groups, filters, occlusion and partial structural damage replay remain future work.
