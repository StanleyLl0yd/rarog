# ADR-0129: Media scheduling is bounded and policy-driven above semantic ownership

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #294

## Context

R5 Audio/video now has bounded semantic resource/stream/playback ownership in `rarog-media` and replaceable demux/decoder/output control-plane adapters in `rarog-media-adapter`. The next boundary must decide when media work is eligible without allowing decoder/device objects, platform handles or generic scheduler correlation IDs to become media authority. Backgrounding and suspension are resource-policy decisions owned by Rarog, while actual HTML autoplay, visibility and user-agent policy remain later Web-platform work.

A long-lived playback also needs bounded work admission. Repeated scheduling while a document is backgrounded, paused or suspended must not accumulate an unbounded queue or revive stale work after policy changes.

## Decision

`rarog-media-runtime` owns a portable `MediaRuntime`. Each runtime has an independent non-zero scope and monotonic `MediaWorkId` serials. Work identities are references only and are never accepted as resource, playback, Host or backend authority. `MediaRuntimeLimits` bounds the combined pending plus active work count.

Every `MediaWork` binds one exact live `MediaPlaybackId` to either an exact selected stream for demux/decode work or an exact selected `MediaStreamKind` for output work. Admission revalidates the playback through `MediaRegistry`, requires `MediaPlaybackState::Playing`, verifies selected-stream/resource ownership, applies the current environment policy and checks duplicate/capacity state before allocating an identity or mutating the queue.

The portable environment is explicit: `Foreground`, `Background` or `Suspended`. Foreground permits otherwise-valid media work. Background uses a configurable `MediaBackgroundPolicy` with independent audio and video eligibility, which allows the current foundation to preserve audio while suppressing video without destroying semantic playback ownership. Suspended permits no new work. These values are engine resource-policy inputs, not claims about complete HTML media, autoplay, page visibility or operating-system lifecycle behavior.

Pending work is FIFO and remains bounded. Before work becomes active, `next_work` revalidates the exact playback/stream relationship and current environment; stale or newly ineligible pending entries are discarded instead of being exported to a backend. Explicit `reconcile` performs the same validation after playback or registry changes. Environment and background-policy transitions reconcile pending work immediately. A transition is rejected while one work item is active so policy cannot change underneath an already exported backend-facing command. Active work must complete explicitly before capacity is recovered.

The runtime deliberately does not reuse the generic `rarog-scheduler` identities as media authority. It also retains no adapter ticket, encoded packet, decoded frame/sample payload, OS handle, device object or target-platform type. Later execution code may map an active `MediaWork` to replaceable adapter operations, but it must continue to treat adapter tickets as private backend correlation state.

## Consequences

Media playback ownership survives backgrounding and suspension while queued execution can be shed deterministically. Resume requires fresh scheduling and therefore fresh work identities rather than reviving stale commands. Queue backpressure is explicit, capacity recovery is exact after completion/cancellation/reconciliation, and independent runtimes cannot alias work references.

This slice intentionally does not define real demux/decoding, encoded/decoded data-plane queues, concrete devices, Windows Media Foundation/WASAPI/DirectShow integration, Host media capabilities, DOM/WebIDL media APIs, MSE/EME/WebAudio, autoplay or visibility-policy completeness, WPT qualification or R6 work. Windows-first backend integration remains the next R5 Audio/video slice.
