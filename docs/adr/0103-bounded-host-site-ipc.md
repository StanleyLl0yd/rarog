# ADR-0103: Bounded Host/Site IPC protocol boundary

Status: accepted

## Context

R4 needs Host/Site communication before selecting a Windows transport. If engine contracts are defined directly in terms of named pipes, sockets, Windows handles or backend serialization objects, process isolation becomes coupled to one platform and transport.

IPC is also an untrusted-input boundary. Site-controlled payloads must be bounded, role-validated and unable to grant themselves process or capability authority.

## Decision

Rarog introduces the dependency-free `rarog-ipc` crate.

The initial protocol owns:

- a current protocol version;
- explicit `Host` and `Site` endpoint roles;
- request, correlated response and event message kinds;
- non-zero request correlation identities;
- owned payloads bounded by explicit message limits;
- per-direction bounded queue count and queued-byte budgets;
- explicit backpressure errors;
- disconnect semantics that discard queued work and reject further send/receive operations.

Only Host↔Site routes are valid. Direct Site↔Site and Host↔Host envelopes fail validation.

An IPC payload cannot self-assert `SiteProcessId` authority. Later Host control-plane code binds a channel/transport endpoint to the Site process that the Host launched and verifies capability references against that Host-owned association.

The portable crate does not choose an encoding, Windows transport or OS handle representation. Received wire data must eventually be decoded into these validated Rarog-owned values under the same limits.

## Consequences

Protocol/lifetime/backpressure semantics can be tested on every platform before Windows transport work starts.

A transport cannot turn an unbounded inbound stream into an unbounded in-memory queue through the Rarog channel contract.

Disconnect removes stale queued work. A later reconnect/replacement path creates new channel/process authority rather than resuming messages from a lost process.

Request IDs provide correlation only and are not capabilities.

Windows named-pipe/shared-memory/socket decisions can change without altering portable engine security semantics.
