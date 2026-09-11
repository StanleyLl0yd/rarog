# ADR-0132: Canvas output revisions use the existing image-resource model

- Status: Accepted
- Date: 2026-09-11
- Tracks: #260, #300

## Context

Canvas output must become visible to paint/compositor invalidation without letting image-resource, paint, compositor or GPU objects become Canvas semantic authority. Rarog already has a bounded `ImageResourceStore` whose `ImageResourceRef` contains a stable resource identity plus a monotonically changing revision.

## Decision

Extend `rarog-canvas` surfaces with bounded portable RGBA output and a monotonic `CanvasContentRevision`. New surfaces allocate a transparent output buffer only after surface/pixel admission succeeds. Output replacement allocates the complete bounded candidate before changing retained pixels or the content revision, so allocation or revision exhaustion leaves the old output intact. `CanvasSurfaceSnapshot` shares an immutable retained pixel allocation and therefore cannot mutate live Canvas state.

Add `rarog-canvas-output` as a separate publication bridge. It owns a bounded mapping from exact `CanvasSurfaceId` values to one `ImageResourceId` each. First publication reserves and resolves one image resource. Later Canvas content revisions update that same resource with `ImageResourceStore::replace_ready`, producing a new `ImageResourceRef` revision. Publishing an unchanged Canvas revision is a no-op. A failed first resolve removes the temporary reservation, and bridge metadata advances only after resource-store replacement succeeds. External resource drift fails closed.

The bridge exposes whether publication changed so the caller can request existing `FrameCause::ResourceReady` scheduling. Paint already records the exact `ImageResourceRef` in `DisplayCommand::DrawImage`; replacing the image resource invalidates the old reference while the new revision resolves the new Canvas pixels. Canvas itself does not depend on resources, paint, compositor or GPU crates.

## Consequences

Canvas output participates in the existing revisioned image path without introducing a second resource namespace or backend authority. Publication and detachment are bounded and recover resource capacity. This slice intentionally provides only whole-surface fill/clear output mutation; broad Canvas drawing APIs, DOM/WebIDL exposure and WebGL remain later R5 work.
