# ADR-0131: Bounded Canvas 2D semantic ownership

- Status: Accepted
- Date: 2026-09-11
- Tracks: #260, #298

## Context

R5 needs Canvas semantics without making paint, compositor, GPU or operating-system objects authoritative Web state. The first Canvas slice must establish bounded surface/context/state lifetime before output revisions or rendering integration exist.

## Decision

Add the portable `rarog-canvas` crate. `CanvasRegistry` owns scoped monotonic `CanvasSurfaceId` and `CanvasContextId` references, bounded live surface/context tables and aggregate pixel accounting. Each surface has non-zero dimensions and may own at most one live 2D context in this slice. A surface cannot retire while that context remains live; retiring the context releases context capacity and allows deterministic surface/pixel-budget recovery. Retired identities are not reused and registry scopes prevent cross-registry aliasing.

`CanvasLimits` bounds live surfaces, live contexts, pixels per surface, aggregate retained surface pixels and saved-state depth. Admission validates all applicable bounds before retained-state mutation. `Canvas2dState` owns only portable fill/stroke colors, global alpha, line width and a finite affine transform. Setters reject invalid values before mutation. `save()` copies the current state only while the configured stack budget permits it; `restore()` on an empty stack is a deterministic no-op.

The crate depends only on `rarog-types`. It owns no pixel buffer, image resource, paint/display-list object, compositor/GPU resource, platform handle, backend ticket or native pointer.

## Consequences

Canvas semantic lifetime and state restoration can now be tested independently from rendering backends, and later output integration can derive work from exact Canvas identities rather than backend objects. Pixel storage/output revisions, paint/compositor invalidation, WebGL, DOM/WebIDL exposure, drawing operations and readback remain later R5 work.
