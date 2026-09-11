# ADR-0125: WebSocket application messages remain bounded Host-owned queue state

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #285

## Context

The R5 WebSocket transport boundary now keeps backend socket identity private behind an exact Host navigation-context Network capability and a Host-owned `WebSocketConnectionId`. The next boundary must retain application messages without allowing a slow transport, a stalled consumer or a malicious backend to create unbounded memory ownership. Outbound backpressure also creates an ownership hazard: removing a message from the Host queue before the backend explicitly accepts it can silently lose Web-visible data.

Inbound flow has the inverse problem. The Host must not ask the backend for arbitrary-sized data after its receive queue is full, and a backend return value cannot be trusted merely because a maximum size was supplied as an argument. Empty WebSocket messages are valid, so an exhausted byte budget is distinct from an exhausted message-count budget: when count capacity remains, a zero-byte message is still admissible.

## Decision

`rarog-websocket::WebSocketMessageQueues` owns two independent FIFO queues per live connection: outbound and inbound. `WebSocketQueueLimits` bounds retained message count and aggregate bytes separately for each direction. Queue accounting uses checked addition before enqueue and checked subtraction before dequeue; failed checks leave queue contents and counters unchanged. `WebSocketMessage` remains the only retained application payload, so UTF-8 text is charged by encoded bytes and binary data by byte length.

The outbound transport contract borrows the oldest queued `WebSocketMessage`. `WebSocketTransportSend::Accepted` is the only result that lets the Host complete and release that queue entry. `Backpressure` and transport errors leave the exact message owned and charged in place. This makes transport handoff lossless without exposing the private `WebSocketTransportTicket` through Host APIs.

For inbound polling, the Host first revalidates the exact navigation-context Network capability and connection authority. `WebSocketMessageQueues::inbound_receive_limit` then derives the maximum acceptable payload size from current count and byte capacity. Exhausted message-count capacity suppresses the backend call entirely. If count capacity remains, a remaining byte budget of zero is represented as an explicit zero-byte receive limit so valid empty messages are not confused with a full queue. The Host rechecks every returned message against the advertised bound before retaining it. A backend that exceeds that bound violates the transport contract; the Host revokes the connection and quarantines its private ticket rather than retaining oversized input.

Queue snapshot, outbound enqueue/flush, inbound poll and inbound dequeue all pass through the existing Host connection-authority validation. Navigation, capability revocation, context close or Site-process loss removes the connection and therefore drops its queued application messages immediately, while the backend ticket remains separately charged in the pending-abort quarantine until cleanup succeeds.

## Consequences

A connection cannot consume another connection's message budget or inspect its queue. Inbound and outbound pressure are independent, rejection is deterministic, and capacity is recovered only when the corresponding owned message is actually removed. Backpressure cannot silently discard outbound data, and a full inbound queue does not cause unnecessary backend reads.

This slice does not define WebSocket frame parsing/masking, ping/pong, selected-protocol or extension negotiation, real HTTP Upgrade/TCP/TLS behavior, close codes/reasons, terminal error propagation or the complete close/error/backpressure state machine. Those remain the final R5 WebSocket lifecycle slice. DOM/WebIDL exposure, WPT qualification and all R6 work remain out of scope.
