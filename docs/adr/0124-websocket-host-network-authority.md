# ADR-0124: WebSocket backend identity remains Host-authorized

- Status: accepted
- Date: 2026-09-11
- R5 tracking: #260, #283

## Context

R5 now has bounded Rarog-owned WebSocket URL, opening-handshake, ready-state and application-message contracts. Those semantic values do not grant network authority and must not expose a native socket, TLS session, backend object or transport ticket to Site-facing code. Rarog already has a Host-owned navigation-context capability model where the exact context, current Site process and broker-issued capability are authoritative, and existing network operations keep backend tickets private behind Host correlation identities.

WebSocket connections are long-lived across backend calls, so revocation has an additional resource-accounting requirement: removing Site-visible authority must not immediately restore Host capacity while the backend connection still requires abort/cleanup. Navigation also requires an explicit document-lifetime rule. A navigation-context Network capability may survive same-site navigation, but a WebSocket opened by the previous document must not silently become authority for the replacement document.

## Decision

`rarog-websocket::WebSocketTransport` is the replaceable backend adapter boundary for this R5 slice. It accepts a validated `WebSocketHandshakeIntent` plus the client `Origin` selected from Host-owned navigation state and returns only a Rarog `WebSocketTransportTicket`. The ticket is backend correlation state, not Site authority; concrete sockets, TLS/session objects and platform handles remain behind the adapter implementation.

`HostControlPlane` allocates a separate scoped monotonic `WebSocketConnectionId`. A live connection binds that reference to the exact navigation context, current Host-resolved `SiteProcessId`, exact Network `CapabilityId`, Host-authenticated client origin and private transport ticket. Opening validates the navigation-context capability class/context/process and client-origin availability, then checks Host connection capacity before calling the transport. The target WebSocket URL may be cross-origin; it does not select or widen client network authority.

Active connections and tickets awaiting backend abort share one configured Host connection budget. Capability revocation, context close, Site-process loss and cross-site replacement revoke the Host-visible connection and move its transport ticket into a private pending-abort quarantine. Every same-site navigation does the same even when the existing Network capability remains live, preventing a connection from crossing the old-document/new-document boundary. Capacity is recovered only after `abort_pending_websocket_connections` confirms backend cleanup; a failed abort leaves the ticket charged.

Transport-ticket reuse across live connections or pending cleanup is treated as backend contract failure. The Host fails closed and revokes affected Host-visible authority rather than allowing two logical connections to alias one backend correlation identity. Independent `HostControlPlane` instances use different connection-ID scopes, so equal serial values do not alias across Host instances.

## Consequences

Site-facing and embedding code receives only `WebSocketConnectionId`; it cannot use the backend ticket as authority or directly address a socket. Rejected capability/context/origin/capacity checks occur before backend start, and lifecycle revocation can remove Web-visible authority without pretending backend cleanup already succeeded.

This slice does not define application send/receive queues, frame parsing or masking, real HTTP Upgrade/TCP/TLS behavior, selected-protocol or extension negotiation, close code/reason semantics, or the complete close/error/backpressure lifecycle. Those remain subsequent R5 WebSocket work. DOM/WebIDL exposure and R6 compatibility qualification remain out of scope.
