# ADR-0012: Stacking context display scopes

## Status

Accepted for R0.

## Context

Rarog already has backend-neutral display commands, explicit clip scopes, retained display-list experiments, and damage-scoped software rasterization. The next paint milestones need a stable structural boundary for CSS stacking, opacity, transforms, and later compositing without coupling those semantics to the software framebuffer.

## Decision

Represent stacking contexts as explicit balanced display-list scopes using `PushStackingContext { id }` and `PopStackingContext` commands. `StackingContextId` is backend-neutral and deterministic within the display-list contract.

Display lists expose a structural-balance invariant that validates proper LIFO nesting across clip and stacking scopes. The software rasterizer currently treats stacking scopes as paint-order-preserving structural markers only.

Damage-scoped rasterization conservatively falls back to a full framebuffer refresh whenever structural scopes are present. This keeps R0 correctness independent of future stacking, clip, transform, opacity, and fragmentation rules.

## Consequences

The display-list contract can gain real stacking order, opacity, transform, and compositor semantics without another structural rewrite. R0 does not claim standards-complete CSS stacking-context behavior yet, and retained damage remains intentionally conservative until those semantics are defined.
