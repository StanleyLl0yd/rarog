# ADR-0116: Windows Storage-process lifecycle and containment

- Status: Accepted
- Date: 2026-09-10

## Context

ADR-0113 established the Rarog-owned `StorageProcessId`, ADR-0114 bound navigation-context Storage authority to exact origins, and ADR-0115 added bounded typed Host-to-Storage request authority. R5 still needs a concrete Windows child lifecycle without turning OS process identifiers, handles, Job Objects, filesystem paths or backend objects into Web authority.

R4 already established the reviewed Windows child-process boundary in `rarog-platform-windows-native::SandboxedChild`. It applies creation-time mitigation attributes, child-process restriction and Job Object containment before child code executes, and exposes only safe lifecycle/evidence operations to `rarog-platform-windows`. A second native boundary would duplicate security-sensitive code without adding authority separation.

Storage loss also differs from Site-process loss. Navigation contexts belong to Site processes and must remain live when the Storage utility child exits, while Storage capabilities minted against the retired Storage generation must not survive recovery.

## Decision

`HostControlPlane` owns Storage-process retirement and recovery.

- `storage_process_lost` accepts only the exact currently-live `StorageProcessId`.
- Before mutating Host state, it validates every tracked Storage capability against its navigation-context owner and broker class.
- A bookkeeping mismatch fails closed before the Storage topology identity is retired.
- On successful loss handling, every tracked Storage capability is revoked, while navigation contexts and non-Storage capability classes remain live.
- The exact Storage topology identity is then retired.
- `recover_storage_process` allocates through the existing monotonic Rarog process-identity allocator, so the replacement is fresh and stale identifiers cannot retire it.

`rarog-platform-windows` adds `WindowsStorageProcess` parallel to the existing Site-process adapter.

- Launch requires a live Host-owned `StorageProcessId` and bounded command arguments.
- Concrete process creation reuses `rarog-platform-windows-native::SandboxedChild`; no new `unsafe` or Win32 boundary is introduced.
- PID, process HANDLE and Job Object HANDLE stay private to the native child abstraction and never become Rarog authority.
- Non-blocking observation, blocking wait and explicit termination report the observed child loss to Host exactly once.
- If Host loss accounting fails after the OS child has exited, the adapter retains its child state so logical retirement can be retried rather than silently dropping Host authority.
- Dropping a live adapter remains emergency containment: it terminates the child but does not fabricate successful Host retirement.
- Non-Windows builds expose an explicit unsupported result rather than emulating Windows process behavior.

## Validation

Coverage must demonstrate:

- bounded Storage-process command arguments;
- explicit non-Windows unsupported behavior;
- stale Storage-process identities rejected at launch and Host retirement;
- Storage loss revokes Storage grants but preserves navigation contexts and unrelated grants;
- inconsistent Storage grant bookkeeping blocks retirement before mutation;
- replacement `StorageProcessId` is fresh and old Storage state cannot authorize against it;
- Windows runtime evidence satisfies the existing R4 sandbox/Job Object policy;
- observed Windows exit is reported once;
- existing Site-process lifecycle and sandbox regressions remain green.

## Consequences

R5 now has a concrete Windows lifecycle and containment boundary for the Storage utility process without widening the native attack surface. This does not claim durability, transactions, filesystem-backed persistence, Web Storage/IndexedDB API completeness, or a concrete authenticated Windows Storage request transport; those remain separate R5 work.
