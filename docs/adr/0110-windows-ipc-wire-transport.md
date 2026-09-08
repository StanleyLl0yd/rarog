# ADR-0110: Bounded Rarog IPC wire codec and Windows local transport

**Status:** Accepted  
**Date:** 2026-09-08

## Context

R4 already defines validated in-memory Host/Site envelopes, Host-owned navigation/process authority and a real Windows child lifetime. Moving envelopes across a process boundary requires a byte representation that cannot trigger unbounded allocation before length validation, plus a Windows transport whose endpoint identity does not become process authority.

## Decision

`rarog-ipc` owns a deterministic 24-byte little-endian fixed header containing `RIPC` magic, protocol version, source/destination roles, request/response/event kind, reserved zero bytes, request correlation identity and u32 payload length.

Header decoding validates limits, magic, version, route, role/kind values, reserved bytes, correlation semantics and payload length before a payload buffer is allocated. Full-frame decoding additionally rejects truncation and trailing bytes. The portable codec has no Windows dependency.

`rarog-platform-windows` supplies the first concrete transport using pinned `interprocess` 2.4.3 with async features disabled. Its Windows local-socket abstraction maps to named pipes while keeping native transport objects private to the platform crate.

A Host listener is created only for a live Host-produced `SiteLease`. An accepted connection stores that lease independently from the decoded envelope. Site→Host and Host→Site direction is checked at the connection API, so forging an envelope role cannot change the connection's Host-owned `SiteProcessId` binding.

Endpoint names are bounded ASCII local tokens. The transport reads and validates the fixed header before allocating and reading exactly the declared payload.

## Consequences

The first real R4 Windows IPC path is bounded before allocation and does not make wire fields, request IDs, pipe names or backend transport objects into Rarog authority.

No named-pipe or Windows handle type enters `rarog-ipc`, `rarog-host` or engine APIs. This completes the Windows-first IPC transport foundation, not the Site sandbox/mitigation policy or final R4 production integration claim.
