# ADR-0130: Windows media backend foundation owns bounded adapter sessions

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #296

## Context

R5 Audio/video now has portable semantic ownership (`rarog-media`), replaceable demux/decoder/output contracts (`rarog-media-adapter`) and a bounded lifecycle-aware scheduling layer (`rarog-media-runtime`). The Windows-first platform still needs a concrete owner for adapter-session lifetime before real Media Foundation, WASAPI or another native implementation can be integrated.

The platform boundary must not make a native object, COM pointer, device handle or adapter ticket authoritative Web state. It must also remain buildable on non-Windows targets so portability CI can detect accidental target leakage.

## Decision

`rarog-platform-windows` owns `WindowsMediaBackend`, a safe-Rust implementation of the existing `MediaDemuxer`, `MediaDecoder` and `MediaOutput` traits. The public surface contains only Rarog media semantic values, portable adapter tickets, fixed errors, limits and count snapshots. It exposes no HANDLE/HWND/COM/Media Foundation/WASAPI/DirectShow/D3D or raw native pointer type.

`WindowsMediaLimits` independently bounds live demux, decoder and output sessions. Capacity is checked before ticket allocation or table mutation. Each successful open receives a fresh process-global monotonic non-zero adapter-ticket value; retired tickets are not reused, and independent backend instances therefore cannot accidentally alias the same correlation value. A ticket has meaning only when it is still present in the exact backend table for its adapter kind. Stale, repeated-close and cross-backend use fails as `InvalidTicket`.

A demux session retains the exact `MediaResourceId`, finite resource duration, selected-stream correlation and current seek position. Stream selection rejects a stream whose semantic resource differs from the opened resource, and seek rejects a position beyond that retained duration. A decoder session retains one exact `MediaStreamId` plus reset position. An output session retains one exact `MediaPlaybackId`, one `MediaStreamKind`, current playback state and position; once that output session is marked `Ended`, the session cannot be revived to a non-ended state. Closing any exact live session releases only that session's bounded capacity.

Public construction is Windows-first and explicit: `WindowsMediaBackend::try_new` succeeds only on a Windows compilation target and returns `UnsupportedTarget` elsewhere. Internal state-machine construction is private and exists only so deterministic portable unit tests can verify ownership rules without pretending that a non-Windows target has a Windows service. Windows CI separately verifies the real target-gated constructor and the same lifecycle tests on `windows-latest`.

The adapter error contract remains fixed and backend-neutral. Capacity/allocator exhaustion is currently classified as the existing generic backend failure rather than widening portable Web semantics with Windows-specific failure detail. Native diagnostic objects or unbounded backend strings are not retained by the portable adapter boundary.

## Consequences

A future native implementation can replace the internals of these exact bounded sessions with Media Foundation/WASAPI or another Windows media stack without changing `rarog-media`, `rarog-media-adapter`, `rarog-media-runtime` or Web-facing identities. Rarog semantic ownership remains upstream of platform execution, while adapter tickets stay correlation references rather than authority.

This foundation deliberately performs no real demuxing, decoding or playback. It defines no encoded packet queue, decoded audio/video payload, device enumeration/selection, capture, DRM/EME, MSE, WebAudio, DOM/WebIDL media element behavior, autoplay-policy completeness, WPT qualification or R6 compatibility claim. Actual native media APIs remain later implementation work behind this established R5 boundary.
