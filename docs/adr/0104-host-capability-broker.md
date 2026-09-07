# ADR-0104: Host-owned capability broker authority

Status: accepted

## Context

R4 Site processes must not receive ambient OS authority. IPC correlation identities and OS transport connections are not sufficient authorization because a compromised Site process can choose arbitrary payload values and request kinds.

Rarog already owns Fetch policy and platform-neutral clipboard contracts. Process loss and replacement also require one place where all authority owned by a stale Site-process identity can be revoked before recovery.

## Decision

Rarog introduces `rarog-broker`, depending on `rarog-process`.

The Host owns a bounded set of capability grants. Each grant contains:

- a monotonically allocated non-zero `CapabilityId`;
- the owning `SiteProcessId`;
- an exact `CapabilityClass`.

The initial classes are `Network` and `Clipboard`, corresponding to existing Rarog-owned privileged boundaries.

A capability ID is an untrusted reference, not authority by possession. Authorization validates that the ID is currently live, belongs to the authenticated Site-process identity associated with the Host-controlled channel, and matches the required class.

Unknown, revoked, wrong-owner and wrong-class references fail closed.

Capability identities are never reused. The broker has an explicit maximum live grant count and fails closed on capacity or identity exhaustion.

The Host can revoke one grant or revoke every grant owned by one Site process. Process-loss handling must perform process-scoped revocation before recovery grants authority to a replacement process.

## Consequences

A compromised Site process cannot mint authority by guessing a capability number or replaying a revoked capability.

IPC request IDs remain correlation-only values and do not become confused with capability identity.

A Network capability authorizes reaching the Host's networking policy boundary; it does not bypass Fetch origin/mode/credentials/redirect policy.

Concrete routing of network, clipboard and future privileged operations through the broker remains a separate integration slice.

No Windows handle, token or other OS authority object crosses the portable broker boundary.
