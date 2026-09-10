# ADR-0113: Storage-process identity and exact-origin bounded storage

Status: accepted

## Context

R5 introduces persistent Web storage after R4 established Host/Site process, navigation-context and capability authority. A Site process is keyed by schemeful site, but persistent storage security is origin-scoped. Reusing Site identity as the storage key would therefore allow distinct same-site origins to share authority incorrectly.

The long-term process topology also places storage in a utility process. That process needs a Rarog-owned identity independent of an OS PID or handle so ownership/lifecycle remains meaningful before and after concrete platform launch integration.

## Decision

`rarog-process` allocates a singleton Host-owned `StorageProcessId` from the same monotonic non-zero engine identity space used for Host/Site process identities. It does not consume the Site-process budget. Retirement removes the live Storage-process identity and recovery allocates a fresh identity; stale storage identities are never reused.

`rarog-storage` owns the initial portable storage-process state model:

- persistent state is keyed by exact `rarog_url::Origin`;
- opaque origins are rejected from persistent storage by default;
- key bytes, value bytes, entries per origin, origin count, bytes per origin and total bytes are explicitly bounded;
- replacements account for the old entry before checking the prospective quota;
- quota/allocation failure occurs before committed mutation so existing values remain intact;
- no filesystem path, file handle or platform database object appears in the public storage contract.

The initial state is in-memory and process-placement-independent. Concrete persistence, transactions, Host/context routing, IPC and Windows Storage-process launch/containment are subsequent R5 slices.

## Consequences

Schemeful-site process sharing cannot become storage sharing accidentally: two same-site but cross-origin documents map to distinct storage areas.

Storage-process ownership can later move behind concrete Windows process/IPC adapters without changing Web storage identity semantics.

This ADR does not claim Web Storage or IndexedDB API completeness, durability or transaction semantics. Site/Web code receives no direct filesystem authority from this foundation.
