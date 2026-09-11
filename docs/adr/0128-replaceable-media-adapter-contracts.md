# ADR-0128: Media demux, decoder and output integration uses replaceable portable adapters

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #292

## Context

The first R5 media slice established bounded Rarog-owned resource, stream and playback identity without codec, device or platform objects. The next boundary must allow concrete demuxers, decoders and output services to be introduced later without making a library context, native handle or operating-system object authoritative media state. The data plane is not ready yet: encoded packets, decoded audio blocks, video frames and scheduler queues need their own ownership and resource policies rather than being invented inside an adapter API.

## Decision

A separate `rarog-media-adapter` crate depends only on `rarog-media` and defines three independent replaceable control-plane traits: `MediaDemuxer`, `MediaDecoder` and `MediaOutput`. Their inputs are immutable Rarog semantic objects and `MediaTime`/state values. No adapter contract exposes container-library types, codec contexts, device handles, OS types or decoded/encoded payload buffers.

Demux control can open one semantic resource, select an exact semantic stream, seek and close. Decoder control can open one exact semantic stream, flush/reset it at a portable media time and close. Output control can open one exact semantic playback/output kind, apply portable playback state/position and close. These are correlation/control contracts only; later orchestration remains responsible for proving that a requested operation is authorized and consistent with live `MediaRegistry` ownership.

Each adapter family returns its own non-zero opaque ticket wrapper. Ticket constructors are public so backend implementations can produce correlation values, but tickets grant no Host, resource, stream, playback or platform authority. A caller must never infer authority from numeric equality or ticket possession. Demux, decoder and output tickets are distinct Rust types even when their raw numeric values match.

Adapter failures retain only the fixed Rarog-owned `MediaAdapterErrorKind` classification (`InvalidTicket`, `Unsupported`, `InvalidState`, `Backend`). Arbitrary backend error strings are deliberately not retained in these contracts. Diagnostics can be layered outside authoritative engine state later with their own bounds.

## Consequences

Concrete libraries and platform implementations can be swapped independently behind the three traits without changing `rarog-media` ownership semantics. Windows-first media work can implement these contracts later while keeping Media Foundation, WASAPI, COM and native handles outside Web/media semantic crates. Tests use replaceable fake adapters and static source checks to prove that exact Rarog resource/stream/playback identities, time, state and kind cross the boundary without backend type leakage.

This slice intentionally does not add encoded packet queues, decoded audio/video payload ownership, device selection, Host/media capability routing, scheduler/background policy, a concrete Windows backend, DOM/WebIDL media APIs, MSE, EME, WebAudio, autoplay policy, WPT qualification or R6 work. Those remain separate R5 slices.
