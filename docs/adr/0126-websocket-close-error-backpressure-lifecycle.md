# ADR-0126: WebSocket closing separates application authority from backend cleanup

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #287

## Context

Rarog already separates WebSocket semantic data, Host connection authority, private transport identity and bounded application-message queues. The final selected R5 WebSocket slice must define what happens when an application requests close, transport backpressure delays progress, the peer/backend stalls, or the transport reports a fatal error. These paths must not revive authority, silently lose retained messages, deliver new inbound data after closing begins, or release connection capacity while backend cleanup is still uncertain.

The portable ready-state enum existed before transport integration, but it did not own legal transitions. Close metadata also needs an explicit hard bound before it crosses the backend boundary. A backend `Closed` observation and a backend error have different resource meaning: confirmed graceful closure can release transport ownership immediately, while an error leaves cleanup uncertain and therefore must remain charged until explicit abort succeeds.

## Decision

`rarog-websocket::WebSocketLifecycle` owns the portable `Connecting -> Open -> Closing -> Closed` transition discipline. `Closed` is terminal and cannot revive. Host construction performs all fallible local setup, including the deterministic `Connecting -> Open` transition, before invoking the backend start operation; a connection becomes Host-visible only after backend start succeeds.

`WebSocketCloseIntent` owns optional local close code plus UTF-8 reason. The hard reason bound is 123 bytes. Local close accepts no code with an empty reason, code 1000, or application/private codes 3000 through 4999. A non-empty reason requires an explicit code. Invalid code/reason/limit input is rejected before any close backend side effect.

A Host close request revalidates the exact navigation-context Network capability and connection authority, then moves the connection to `Closing` without touching the backend. This immediately rejects new application-message enqueue and suppresses further inbound backend polling. Messages that were already retained are not silently lost: queued outbound messages may continue to flush while Closing and retryable `WebSocketTransportSend::Backpressure` preserves the exact oldest payload and charge; already queued inbound messages may still be consumed until terminal closure.

The Host does not start the backend close handshake until the outbound FIFO is empty. `WebSocketTransportCloseStart::Backpressure` is retryable and means the backend did not accept close ownership; the Host keeps the same close intent and remains Closing. `Started` records that backend close progress may now be polled. `WebSocketTransportClosePoll::Pending` retains the connection and its capacity charge. `Closed` is a backend guarantee that the underlying connection is fully closed and needs no later abort; the Host transitions terminal, clears application queues and removes the connection exactly once.

A transport error from send, receive, begin-close or close-poll is terminal rather than retryable backpressure. The Host marks the lifecycle Closed, clears both application queues, removes ordinary connection authority and places the private transport ticket into the existing pending-abort quarantine. Active/Closing connections and quarantined tickets share the configured connection budget, so an error cannot manufacture fresh capacity before cleanup succeeds. Navigation, capability revocation, context close and Site-process loss remain stronger authority-loss paths and converge on the same private-ticket quarantine without exposing or reusing the `WebSocketConnectionId`.

## Consequences

Backpressure and fatal errors now have deliberately different semantics: backpressure retains ordinary live authority and queued data, while a backend error terminates ordinary authority and preserves only cleanup ownership. Graceful close completion releases capacity without an unnecessary abort because the backend has explicitly confirmed closure. Repeated close with the same intent is idempotent before backend start; changing close intent after Closing begins is rejected. Repeated operations after terminal removal fail before backend access.

This R5 lifecycle does not implement WebSocket frame parsing/masking, ping/pong scheduling, compression/extensions, complete peer close-frame metadata, real HTTP Upgrade/TCP/TLS behavior, DOM/WebIDL exposure, WPT qualification or R6 compatibility claims. Those remain outside this selected R5 boundary.
