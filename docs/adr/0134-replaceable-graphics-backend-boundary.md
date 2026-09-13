# ADR-0134: Replaceable graphics backend boundary

Status: Accepted

## Context

R5 needs a graphics integration boundary after the bounded Canvas/WebGL semantic model. Real graphics implementations own devices, queues, buffers, textures, views and platform/native state, but none of those objects may become Web, DOM, Canvas or WebGL authority. The engine also needs deterministic behavior when backend allocation, destruction or device lifetime fails.

A simple mapping from semantic resource IDs to backend objects is insufficient if cleanup failure silently frees adapter capacity: repeated semantic create/destroy cycles could otherwise accumulate unreachable backend state without bound. Backend loss also must not leak native error objects into Web-visible state or invent a second context-loss model.

## Decision

- `rarog-graphics-adapter` is a portable integration crate depending only on `rarog-canvas` and `rarog-webgl`. It does not depend on `wgpu`, D3D/DXGI, platform crates or native handles.
- `GraphicsBackend` is the replaceable backend contract. Concrete context/buffer/texture handle types are associated backend types retained only inside `GraphicsAdapter`; callers never receive them and backend handles are never accepted as semantic authority.
- The adapter validates exact live `WebGlContextId`, `WebGlBufferId` and `WebGlTextureId` ownership before backend creation. Cross-context, stale and cross-registry semantic identities fail closed before backend mutation.
- Backend creation errors are reduced to fixed `GraphicsBackendErrorKind` classifications. A conforming backend wrapper must make failed create operations atomic from the adapter's perspective: it either returns a retained handle or leaves no unreachable backend allocation.
- Adapter context/resource binding counts are independently bounded. Cleanup-pending bindings continue to consume those limits until backend cleanup succeeds.
- Semantic resource/context destruction retires WebGL authority first. If backend destruction then fails, the private handle remains retained as `CleanupPending` and therefore remains charged against adapter capacity. Explicit retry is required before that backend capacity is recovered.
- Backend context loss maps only to the engine-owned `WebGlContextLossReason` values. Loss first transitions the semantic WebGL context and retires its semantic resources; correlated backend resources are then cleaned or retained as bounded cleanup-pending state.
- A context backend handle is not destroyed while correlated backend resource handles remain retained. Context cleanup failure likewise keeps the context binding charged until retry succeeds.
- Public adapter observations expose only semantic IDs, binding state, counts, limits and fixed error classifications. There is no public backend ticket, native pointer, device, queue, buffer, texture, view, shader, pipeline or command object.
- Mock and no-op backend implementations exercise the same adapter contract in portable tests, proving replacement without selecting or claiming a production GPU implementation.

## Consequences

- Canvas/WebGL semantic authority remains independent from backend object lifetime and from the chosen graphics implementation.
- Backend create failures cannot manufacture adapter bindings, while backend destroy failures cannot manufacture backend capacity because cleanup-pending handles remain bounded and charged.
- Stale backend objects cannot resurrect retired WebGL IDs or authorize operations on replacement semantic resources.
- Backend device/reset/resource-pressure events use the existing WebGL loss model instead of leaking platform-specific error state upward.
- A future Windows-first `wgpu` or native graphics implementation can implement `GraphicsBackend` behind this boundary without changing Web/DOM contracts.

## Non-goals

This decision does not add WebGL command encoding, shaders/programs, framebuffers, draw calls, GPU upload/readback, extension/WebGL2 completeness, a production `wgpu` WebGL backend, DOM/WebIDL exposure, WPT qualification, compatibility claims, Accessibility implementation or any R6 work.
