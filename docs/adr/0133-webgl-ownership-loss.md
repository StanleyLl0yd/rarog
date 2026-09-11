# ADR-0133: WebGL semantic ownership and context loss

Status: Accepted

## Context

R5 needs a WebGL ownership foundation without allowing Web/DOM-visible state to become GPU, graphics-backend, platform-handle or native-object authority. Canvas already owns bounded surface identity and output revisions, so WebGL must share exact Canvas surface exclusivity while keeping its own semantic context/resource lifetime and resource-pressure accounting.

A lost graphics context also needs deterministic fail-closed semantics. Resource identities must stop authorizing mutation immediately, accounted capacity must recover in a bounded operation, and the Canvas surface must not silently switch to another rendering-context kind before WebGL cleanup is complete.

## Decision

- `rarog-canvas` owns exact live Canvas surface identity and rendering-context exclusivity. A WebGL context first acquires a scoped monotonic `CanvasExternalContextLease` for one exact live `CanvasSurfaceId`.
- A surface with a live 2D context cannot acquire an external WebGL lease, and a surface with a live external lease cannot create a 2D context or retire the surface.
- `rarog-webgl` is a portable semantic crate depending only on `rarog-canvas`. It owns scoped monotonic `WebGlContextId`, `WebGlBufferId` and `WebGlTextureId` values; those values are never GPU/native/backend handles.
- `WebGlLimits` bounds live contexts, resources per context, aggregate resources, bytes per buffer, aggregate buffer bytes, texture dimensions, pixels per texture and aggregate texture pixels. Count/byte/pixel arithmetic is checked before retained-state mutation.
- Each buffer or texture is owned by exactly one live WebGL context. Foreign-context and cross-registry identities fail closed.
- Context state is explicitly `Active` or `Lost(WebGlContextLossReason)`. Loss reasons are fixed engine-owned classifications rather than backend/native error objects.
- Context loss retires all resource authority for that context and releases its accounted buffer/texture budgets in one operation bounded by the global resource limits. The lost context and its exact Canvas external lease remain retained for deterministic cleanup.
- Context destruction retires any remaining resources, revalidates the exact Canvas lease, releases that lease and then releases WebGL context capacity. Stale resource/context identities are not reused or resurrected.
- Public contracts expose only immutable semantic views, limits, counts and fixed loss classifications. GPU devices, queues, buffers, textures, views, shaders, programs, pipelines, command encoders and native pointers remain outside this boundary.

## Consequences

- Canvas 2D and WebGL now have one exact, fail-closed surface-exclusivity boundary without making WebGL or a future graphics backend authoritative over Canvas surface identity.
- Resource pressure is deterministic and bounded before a real graphics backend exists.
- Lost contexts cannot create new resources, while deterministic destruction can still release the Canvas surface later.
- A future replaceable graphics backend can correlate private backend objects with these semantic identities, but it must not expose those objects as Web/DOM authority.

## Non-goals

This decision does not add GL command encoding, shaders/programs, framebuffers, draw calls, GPU upload/readback, WebGL extensions, WebGL2 completeness, `wgpu`, D3D/DXGI/native graphics objects, DOM/WebIDL exposure, WPT qualification or any R6 work. The next R5 Canvas/WebGL slice remains the replaceable graphics-backend boundary proving backend objects stay outside Web/DOM contracts.
