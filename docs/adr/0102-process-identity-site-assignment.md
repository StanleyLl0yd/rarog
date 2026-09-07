# ADR-0102: Process identity and site-assignment boundary

Status: accepted

## Context

ADR-0003 requires production Rarog to separate mutually untrusted sites at a process boundary. R2 already established Rarog-owned `Origin` and schemeful `SiteIdentity`; R3 established owned frame packets that can cross execution/lifetime boundaries. R4 now needs process ownership without making OS process IDs, Windows handles or a particular IPC transport part of engine security semantics.

A process budget is also not permission to weaken site isolation. If the Host cannot allocate another Site-process slot, assigning a distinct site to an existing process would silently widen authority.

## Decision

Rarog introduces `rarog-process`, depending only on `rarog-url`.

The Host owns one engine-level `HostProcessId` plus monotonically allocated `SiteProcessId` values. These are Rarog identities, not OS PIDs or handles.

`ProcessTopology` owns a bounded mapping between `SiteIdentity` and live `SiteProcessId` values:

- the exact same schemeful site may reuse its existing assignment;
- a distinct site requires a distinct Site-process identity;
- an opaque site reuses authority only when the exact opaque identity is propagated;
- reaching the configured Site-process limit returns an error;
- no cross-site fallback process exists;
- retiring a Site-process assignment removes the mapping;
- retired process identities are never reused by the allocator.

The first slice models logical process placement only. OS launch state, IPC channels, sandbox tokens/handles, capability broker state and crash-recovery policy will compose with these identities in later R4 slices rather than being embedded in the topology.

## Consequences

Process placement can become multi-process without changing URL/origin/site identity semantics.

IPC and capability messages can later bind authority to Rarog process identities rather than trusting OS or transport identifiers directly.

A memory/process-pressure policy must reject, retire or otherwise explicitly handle work when capacity is exhausted; it cannot silently merge mutually untrusted sites.

Windows-specific process and sandbox implementation remains replaceable behind a narrow platform boundary.

The Host/Site identity space is bounded by non-zero 64-bit values and fails closed on exhaustion.
