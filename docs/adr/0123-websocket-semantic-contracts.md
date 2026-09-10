# ADR-0123: WebSocket semantic contracts remain backend-neutral

- Status: accepted
- Date: 2026-09-10
- R5 tracking: #260, #281

## Context

R5 needs WebSocket semantics before it can safely attach WebSocket work to Host/network authority. The Web-facing URL, requested subprotocols, ready state and application messages are distinct from TCP/TLS/socket handles, HTTP Upgrade I/O, wire frames and backend connection identities. Mixing those layers would let transport choices leak into Web semantics and would make later capability revocation and queue accounting harder to enforce.

The WebSocket constructor model accepts `ws`/`wss` URLs and maps `http`/`https` to those schemes, rejects URL fragments, preserves the ordered requested subprotocol list while rejecting duplicate or invalid HTTP-token values, and exposes the four connecting/open/closing/closed ready states. Application text and binary messages are semantic payloads rather than wire frames.

## Decision

Add `rarog-websocket` as a portable Rarog-owned contract crate depending only on `rarog-url`. `WebSocketLimits` bounds retained canonical URL bytes, requested subprotocol count, bytes per requested subprotocol and bytes per application message. Zero-valued limits are rejected.

`WebSocketUrl` wraps a Rarog `WebUrl`. Raw input is byte-bounded before URL parsing, `http` is normalized to `ws`, `https` to `wss`, existing `ws`/`wss` is retained, other schemes and any fragment are rejected, and the canonical serialized URL is checked again before retention. Resource-name derivation exposes only path plus optional query; secure state derives from `wss`.

`WebSocketProtocols` owns an ordered vector of private `WebSocketSubprotocol` values. Each candidate is length-checked and validated as a non-empty HTTP token before it is copied; duplicates are rejected with case-sensitive string identity and count is bounded before per-item ownership. `WebSocketHandshakeIntent` owns only the validated canonical URL and requested protocols. It deliberately does not generate `Sec-WebSocket-Key`, randomness, raw HTTP headers, extensions or backend connection identities.

`WebSocketReadyState` represents `Connecting`, `Open`, `Closing` and `Closed` with the Web-facing numeric values 0 through 3, but does not itself perform transport transitions. `WebSocketMessage` owns either text or binary application data behind a private payload representation. Text accounting uses UTF-8 bytes; text and binary constructors reject oversize data before copying it into retained storage.

## Consequences

The first WebSocket R5 slice is deterministic, bounded and platform-neutral. No socket descriptor, native handle, backend ticket, frame opcode, masking key or platform type appears in the public contract. A later Host/network slice can therefore bind these validated semantic values to authenticated navigation/process/capability ownership without making the backend object authoritative.

This ADR does not claim actual WebSocket opening-handshake I/O, Fetch/HTTP integration, DNS/TCP/TLS, frame parsing/serialization, masking, send/receive queues, close/error/backpressure lifecycle, DOM/WebIDL exposure or compatibility qualification. Those remain later R5/R6 work and must not be inferred from these semantic types.
