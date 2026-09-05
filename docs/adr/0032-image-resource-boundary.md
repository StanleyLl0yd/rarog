# ADR-0032: Bounded image resource boundary

**Status:** Accepted

## Context

R1 needs an image resource abstraction before later URL/Fetch and asynchronous decoder work exists. Decoded image ownership must not live in DOM/layout, and paint must not depend on a network stack, platform graphics API, decoder library, or hidden process-global cache. Resource-content changes also need explicit identity so retained display lists and damage tracking cannot silently reuse stale pixels.

## Decision

Add a platform-neutral `rarog-resources` crate. It owns:

- monotonic typed `ImageResourceId` values within one store;
- revisioned `ImageResourceRef` snapshots for resource state, including pending revision 0;
- pending, ready and failed lifecycle states;
- RGBA8 `DecodedImage` buffers with exact dimension/pixel-count validation;
- explicit limits for resource count, pixels per resource and total retained decoded pixels.

Ready resolution and pixel replacement advance the resource revision. Old references become stale and do not resolve. Removal releases retained pixel budget and IDs are not reused.

The store may expose the current revision snapshot for a pending resource before decoded pixels exist. Such a reference resolves to no image and therefore paints nothing. This lets retained display state name the resource while asynchronous decode is pending; when completion advances the revision, the display command must be refreshed to the newer reference so normal damage comparison observes the transition.

`rarog-paint` adds a backend-neutral `DrawImage` display command containing destination geometry and an `ImageResourceRef`. Rasterization receives an image store explicitly. Pending, missing or stale references paint no content. The resource revision is part of display-command equality/snapshots, so content changes are visible to normal damage comparison rather than hidden behind mutable cache state.

## Consequences

- DOM/layout remain free of decoded pixel ownership and decoder/network types.
- Paint can consume resolved images without acquiring OS or network capabilities.
- Resource retention is bounded without introducing a process-global immortal cache.
- Future image decoders can be replaced behind the decoded-image boundary.
- Future resource updates can invalidate paint through explicit revisions.
- This ADR does not implement URL resolution, Fetch, image format decoding, HTML `<img>` replaced-element semantics, responsive images, animation or asynchronous decode. Those remain later roadmap work.
