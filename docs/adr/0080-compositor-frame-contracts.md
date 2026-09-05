# ADR-0080: Backend-neutral compositor frame contracts

## Status

Accepted.

## Context

R3 needs a compositor, GPU backend and frame scheduler. Binding engine/paint directly to `wgpu` surface, device or texture types would make the graphics implementation part of browser-engine contracts and make CPU/headless/testing paths harder to preserve.

Paint already owns stable display-list identities and deterministic damage calculation. The compositor boundary should consume those results without moving graphics-backend ownership into paint.

## Decision

Rarog introduces a dedicated `rarog-compositor` crate that depends only on Rarog paint/types.

The first boundary defines:

- `SurfaceId`: non-zero engine-owned surface identity;
- `SurfaceSize`: physical pixel extent, including an explicit zero-size suspended state;
- `FrameId`: planner-owned monotonically increasing presentation work identity;
- `DisplayListRevision`: engine-owned scene/display-list revision metadata;
- `FrameCause`: initial, resize, scene, scroll, resource and explicit frame causes;
- `FrameUpdateKind`: full or partial redraw;
- `FramePlan`: validated surface/revision/cause/damage metadata;
- `FrameDecision`: no-op, suspended or submit;
- `FrameSubmission`: a borrowed validated plan plus Rarog `DisplayList` and backend-neutral clear color;
- `CompositorBackend`: backend-neutral submission trait.

`FramePlanner` is per-surface and permits one pending frame.

Planning rules:

1. reject a new plan while another frame is pending;
2. validate all input damage before allocating frame identity;
3. a zero-width or zero-height surface is suspended and clears presented state;
4. the first active frame and every surface-size change require a full surface redraw;
5. otherwise paint damage is clipped to the active surface and exact duplicate rectangles are removed;
6. empty clipped damage produces no frame;
7. damage equal to the complete surface may be promoted to a full update;
8. partial damage preserves the caller's frame cause;
9. submitted work must be explicitly completed or discarded;
10. only successful completion advances the remembered presented size/revision;
11. discarded IDs are never reused;
12. identity exhaustion fails closed.

The planner does not own threads, timing, window handles, swap chains, GPU resources or device state.

## Consequences

A future `wgpu` implementation can be replaced without changing DOM/CSS/layout/paint APIs.

CPU/headless and deterministic test backends can implement the same submission contract.

Engine frame scheduling can reason about pending/presented work independently from graphics-library lifetime rules.

Damage is normalized once at the compositor boundary before reaching backend-specific code.

The submission carries the clear/background `Color` required to reproduce CPU full/partial raster semantics without importing engine options into a graphics backend. A future mutable background-color API must invalidate the surface appropriately; the current render background is immutable within a session.

## Deferred

Later R3 slices own:

- engine integration and display-list revision lifecycle;
- compositor thread/task ownership;
- `wgpu` adapter/device/surface implementation;
- Windows native surface integration;
- retained GPU scene/resource caches;
- scroll tree and asynchronous resource completion;
- frame pacing and presentation feedback.
