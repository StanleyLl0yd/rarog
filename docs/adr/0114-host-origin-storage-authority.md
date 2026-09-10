# ADR-0114: Host exact-origin storage authority

Status: accepted

## Context

R4 navigation contexts deliberately tracked schemeful-site process placement and context-scoped capabilities. R5 persistent storage is stricter: two documents may share one Site process while remaining cross-origin and therefore must not share storage authority.

Deriving the storage origin from a Site process, a capability number, or an untrusted request field would allow authority confusion. A capability granted to one document must also not become valid for another same-site cross-origin document after navigation.

## Decision

URL-backed `HostControlPlane` navigation contexts retain the exact current `rarog_url::Origin` in addition to their existing `DocumentSiteBinding`. The origin is derived once from the target URL and its `SiteIdentity` is derived from that same origin, which is especially important for opaque-origin identity.

The lower-level `open_navigation_context_to_site` / `navigate_navigation_context_to_site` APIs remain explicitly originless. They can exercise topology and trusted embedding paths, but they cannot mint persistent Storage authority.

`CapabilityClass::Storage` is valid only through a live navigation context with a non-opaque exact origin. Each Storage capability is privately bound by the Host to the exact origin current at grant time. Storage reads and mutations require, in order:

1. Storage class and context ownership;
2. live broker authorization for the current Site process;
3. equality between the current navigation-context origin and the private grant origin;
4. equality between the supplied storage state and the live Host-owned `StorageProcessId`;
5. the bounded quota checks owned by `rarog-storage`.

Same-site cross-origin navigation updates the context origin and revokes its Storage capabilities. Even if revocation cleanup were incomplete, the changed exact-origin comparison fails closed before data access. Cross-site navigation retains the existing R4 behavior of revoking all moving-context capabilities.

No filesystem path, OS process identifier, backend database handle or Site-supplied origin participates in this authority decision.

## Consequences

A Site process may safely host multiple same-site origins without becoming a storage security principal. Storage authority is a conjunction of Host-owned process, navigation-context, capability, exact-origin and Storage-process identities.

The initial route still calls the in-memory `StorageProcessState` directly. Storage IPC, durable persistence and concrete Windows Storage-process launch/containment remain later R5 slices.
