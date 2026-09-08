# R4 — Sky backlog

Status: **in progress**.

Tracking issue: #231.

## A — Process topology

- [x] Introduce Rarog-owned Host/Site process identities independent of OS process IDs and handles.
- [x] Add bounded Host-owned schemeful-site assignment with explicit reuse only for the same `SiteIdentity`.
- [x] Fail closed when the Site-process budget is exhausted instead of sharing a process across distinct sites.
- [x] Retire Site-process assignments without reusing stale process identities.
- [x] Add bounded Host-owned navigation contexts with explicit Site-process assignment and replacement policy (#239).
- [ ] Wire embedder document/navigation entry points to Host-owned navigation contexts.

## B — IPC

- [x] Define versioned Rarog-owned IPC endpoint roles, message envelopes and protocol limits (#233).
- [x] Add request/reply correlation, validation and bounded queue/backpressure semantics (#233).
- [x] Define transport-independent disconnect/protocol-error behavior (#233).
- [ ] Add a Windows-first IPC transport behind the portable protocol boundary.

## C — Capability broker

- [x] Define Host-owned brokered capability identities, classes and revocation (#234).
- [x] Bind capability authority to the owning Site-process identity and provide process-scoped revocation (#234).
- [ ] Route privileged network/platform operations through explicit broker checks.
- [x] Prove numeric references, wrong-owner/class use and revoked capability reuse fail closed (#234).

## D — Windows process hardening

- [ ] Define the narrow Windows Host/Site process launch adapter.
- [ ] Apply Windows-first process mitigation/sandbox policy for Site processes.
- [ ] Keep Windows handles, tokens, job objects and mitigation APIs outside portable engine crates.
- [ ] Add Windows-specific process/sandbox validation to CI.

## E — Site isolation integration

- [x] Make cross-site process separation the default Host navigation policy (#239).
- [x] Define same-site reuse, shared-site retention and cross-site replacement rules (#239).
- [x] Preserve opaque-origin/site isolation through explicit environment-owned identity propagation (#239).
- [x] Connect Site-process authority to Host-bound IPC and broker ownership without address-space assumptions (#236).

## F — Crash recovery

- [ ] Detect Site-process loss and invalidate the corresponding process/channel state.
- [x] Revoke all brokered capabilities owned by a lost Site process before retirement/recovery (#236).
- [x] Recreate logical Site state with a fresh process identity and empty channel authority (#236).
- [x] Reject stale process/channel/capability authority after loss and replacement (#236).

## Closure

R4 closes only after the bounded Sky scope is integrated into production paths, `docs/R4-EXIT.md` is complete, a dedicated `r4_exit` integration gate is present, and Windows-primary/Linux-portability/security CI are green.

R4 completion will not imply a complete browser sandbox, storage/worker/media support, general-Web compatibility or browser readiness.
