# R4 — Sky backlog

Status: **complete**.

Tracking issue: #231.

## A — Process topology

- [x] Introduce Rarog-owned Host/Site process identities independent of OS process IDs and handles.
- [x] Add bounded Host-owned schemeful-site assignment with explicit reuse only for the same `SiteIdentity`.
- [x] Fail closed when the Site-process budget is exhausted instead of sharing a process across distinct sites.
- [x] Retire Site-process assignments without reusing stale process identities.
- [x] Connect document/navigation lifecycle to explicit Site-process assignment and replacement (#241).
- [x] Add bounded Host-owned navigation contexts with last-reference Site-process lifetime and fail-closed one-slot replacement (#248).

## B — IPC

- [x] Define versioned Rarog-owned IPC endpoint roles, message envelopes and protocol limits (#233).
- [x] Add request/reply correlation, validation and bounded queue/backpressure semantics (#233).
- [x] Define transport-independent disconnect/protocol-error behavior (#233).
- [x] Add a bounded Windows-first IPC wire codec/transport behind the portable protocol boundary (#250).

## C — Capability broker

- [x] Define Host-owned brokered capability identities, classes and revocation (#234).
- [x] Bind capability authority to the owning Site-process identity and provide process-scoped revocation (#234).
- [x] Route privileged Network/Clipboard operations through explicit broker checks with Host-owned network-operation authority (#243).
- [x] Scope privileged capability handles to exact Host navigation contexts so same-process contexts cannot interchange authority (#248).
- [x] Prove numeric references, wrong-owner/class use and revoked capability reuse fail closed (#234).

## D — Windows process hardening

- [x] Define the narrow Windows Host/Site process launch/loss adapter (#245).
- [x] Apply the bounded Windows-first process mitigation/sandbox policy for Site processes (#254).
- [x] Keep Windows handles, tokens, job objects and mitigation APIs outside portable engine crates through the dedicated native boundary (#254).
- [x] Add Windows-specific process/sandbox validation to CI (#254).

## E — Site isolation integration

- [x] Make cross-site process separation the default navigation policy (#241).
- [x] Define same-site reuse and cross-site replacement rules (#241).
- [x] Preserve opaque-origin/site isolation through explicit environment-owned identity propagation (#241).
- [x] Connect Site-process authority to Host-bound IPC and broker ownership without address-space assumptions (#236).
- [x] Resolve production navigation authority from Host context state rather than embedder-supplied Site-process identity (#248).

## F — Crash recovery

- [x] Detect Windows Site-process loss and invalidate the corresponding process/channel state (#245).
- [x] Revoke all brokered capabilities owned by a lost Site process before retirement/recovery (#236).
- [x] Recreate logical Site state with a fresh process identity and empty channel authority (#236).
- [x] Reject stale process/channel/capability authority after loss and replacement (#236).

## Closure

R4 closes with the bounded Sky scope integrated into the production-facing authority paths, `docs/R4-EXIT.md` complete, the dedicated `r4_exit` integration gate present on Windows and Linux, and Windows-primary/Linux-portability/security CI green.

R4 completion will not imply a complete browser sandbox, storage/worker/media support, general-Web compatibility or browser readiness.
