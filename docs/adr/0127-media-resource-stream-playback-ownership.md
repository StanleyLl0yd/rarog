# ADR-0127: Media resource, stream and playback ownership is portable and bounded

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #289

## Context

R5 Audio/video needs an engine-owned semantic lifetime model before decoder, demuxer, audio-device, video-surface or operating-system backends are introduced. If platform or codec objects become the first source of identity, later Host authorization, scheduling and Windows-first adapters would inherit backend lifetime rules into Web semantics. Media input is also resource-sensitive: a resource may expose many streams, metadata may request extreme channel/sample-rate or video dimensions, and stale playback references must not keep retired objects implicitly alive.

The first slice therefore needs only enough state to describe bounded media ownership and legal playback lifetime. It must not guess codec support, encoded-buffer ownership, device selection, autoplay policy or DOM behavior that belong to later slices.

## Decision

A dependency-free `rarog-media` crate owns `MediaRegistry`, `MediaResource`, `MediaStream` and `MediaPlayback`. Each registry receives a fresh process-local non-zero scope. Resource, stream and playback identities combine that scope with independent monotonic non-zero serials. The identities are correlation references only; they contain no Host capability, process identity, platform handle or backend ticket, and retired serials are never reused.

`MediaLimits` bounds live resources, total retained streams, streams per resource, live playback sessions, audio channel count/sample rate and video dimensions. Limits are non-zero and internally consistent. `MediaResourceDescriptor` validates metadata before copying it, and `MediaRegistry::create_resource` revalidates the descriptor against the registry's own limits so data constructed under a wider policy cannot bypass the destination registry. Aggregate resource, stream and playback counts use checked arithmetic before mutation. Ordinary validation/capacity failures leave retained ownership unchanged.

A resource owns a finite `MediaTime` duration expressed as unsigned integer microseconds plus one or more exact stream identities. Audio streams retain only bounded channel count and sample rate; video streams retain only bounded dimensions. This foundation has no codec/container/backend strings or decoded/encoded buffers. Integer time deliberately has no NaN or infinity sentinel. Playback position must remain at or below the exact resource duration.

A playback session binds one exact live resource to a non-empty, duplicate-free list of exact streams owned by that resource. A foreign, stale or differently owned stream fails closed. The portable lifecycle is `Ready`, `Playing`, `Paused`, `Ended`: Ready or Paused may enter Playing, Playing may pause, and Ended is terminal for that playback identity. Marking a session ended fixes its position at the exact resource duration. A fresh playback identity is required for new playback after terminal state.

A live playback prevents retirement of its resource. Retiring a playback releases playback capacity; retiring an unused resource validates all retained stream ownership, then removes the resource and its streams together so stream capacity is recovered deterministically. Read APIs expose immutable references or count snapshots only; registry maps remain private.

## Consequences

Later demux/codec/device/platform adapters can correlate their own work with these Rarog references without becoming authoritative owners of media semantics. Later scheduling/background policy can act on explicit playback state instead of inferring lifetime from decoder or device handles. Cross-registry identities naturally fail because their scope differs, while stale identities fail after retirement because map ownership is gone and serials are never reused.

This slice intentionally does not define demuxers, codecs, encoded or decoded sample queues, audio devices, video surfaces, platform media services, Windows Media Foundation/DirectShow/WASAPI objects, backend tickets, scheduler/background policy, DOM/WebIDL media elements, MSE, EME, WebAudio, autoplay policy, real decoding/playback, WPT qualification or any R6 compatibility claim. Those remain separate R5 work.
