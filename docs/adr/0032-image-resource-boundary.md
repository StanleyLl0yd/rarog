# ADR-0032: Bounded image resource boundary

**Status:** Accepted

## Context

R1 needs an image resource abstraction before later URL/Fetch and asynchronous decoder work exists. Decoded image ownership must not live in DOM/layout, and paint must not depend on a network stack, platform graphics API, decoder library, or hidden process-global cache. Resource-content changes also need explicit identity so retained display lists and damage tracking cannot silently reuse stale pixels.

## Decision

Add a platform-neutral `rarog-resources` crate. It owns:

- monotonic typed `ImageResourceId` values within one store;
- revisioned `ImageResourceRef` snapshots for ready decoded content;
- pending, ready and failed lifecycle states;
- RGBA8 `DecodedImage` buffers with exact dimension/pixel-count validation;
- explicit limits for resource count, pixels per resource and total retained decoded pixels.

Ready pixel replacement advances the resource revision. Old references become stale and do not resolve. Removal releases retained pixel budget and IDs are not reused.

`rarog-paint` adds a backend-neutral `DrawImage` display command containing destination geometry and an `ImageResourceRef`. Rasterization receives an image store explicitly. Missing or stale references paint no content. The resource revision is part of display-command equality/snapshots, so content changes are visible to normal damage comparison rather than hidden behind mutable cache state.

## Consequences

- DOM/layout remain free of decoded pixel ownership and decoder/network types.
- Paint can consume resolved images without acquiring OS or network capabilities.
- Resource retention is bounded without introducing a process-global immortal cache.
- Future image decoders can be replaced behind the decoded-image boundary.
- Future resource updates can invalidate paint through explicit revisions.
- This ADR does not implement URL resolution, Fetch, image format decoding, HTML `<img>` replaced-element semantics, responsive images, animation or asynchronous decode. Those remain later roadmap work.
