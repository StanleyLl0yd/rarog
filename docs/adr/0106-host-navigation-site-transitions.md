# ADR-0106: Host-owned navigation and Site-process transitions

Status: accepted

## Context

R4 process topology can assign a `SiteIdentity` to a Site-process identity, and the Host control plane can revoke stale channel/capability authority. Navigation still needs an explicit ownership model. Treating an engine view ID, URL string, OS process ID or IPC payload as navigation authority would couple Web security identity to the wrong layer and make cross-site replacement ambiguous.

A process budget also creates a lifecycle edge case: with one available Site-process slot, a sole document must be able to navigate cross-site without sharing mutually untrusted sites or permanently failing only because its old process still occupies the slot.

Opaque origins add another requirement. Re-deriving a `SiteIdentity` from an opaque URL creates a fresh opaque identity, so the exact environment-owned identity must be propagated when reuse is intended.

## Decision

`rarog-host` owns a bounded `NavigationControlPlane` above `HostControlPlane`.

Each live navigation context has a monotonically allocated, non-zero `NavigationContextId` and one Host-owned `SiteProcessId` binding. Context identities are never reused after close.

The controller accepts `SiteIdentity` values rather than URL strings:

- exact same-site identity keeps the current process;
- an already-live target site reuses that target process;
- a new cross-site target receives a distinct process;
- the source process remains live while any other navigation context still references it;
- once unreferenced, source cleanup uses the Host control plane's disconnect → revoke-all → retire order.

If a new cross-site target cannot be allocated because the process budget is full and the moving context is the source process's only reference, the controller performs a fail-closed capacity replacement: source authority is disconnected, revoked and retired before a fresh target process is created. A failure after source retirement invalidates the context instead of restoring stale authority or sharing a process across sites.

Capability operations exposed by the navigation controller resolve ownership from the Host-held context binding. Callers do not provide a process owner to widen authority.

Opaque site identity is explicit input. The controller never serializes and re-parses it. Reuse therefore requires propagation of the exact opaque `SiteIdentity`; independently derived opaque identities remain isolated.

## Consequences

Navigation lifecycle now has a bounded, process-aware Host contract that can be wired to embedder document/navigation entry points without making `ViewId`, OS PID/handles or transport state authoritative.

A cross-site move can revoke the moving context's stale capability authority when its source process is retired. If the source process is shared by another same-site context, that process and its grants remain live for the remaining owner while the moved context fails owner checks against them.

The controller may temporarily hold both source and target processes when capacity permits so target creation can fail without disturbing the current context. The one-slot fallback deliberately prioritizes isolation and stale-authority revocation over preserving a failed navigation's old process.

This ADR does not wire the engine/embedder navigation callback path, launch an OS process, choose an IPC transport or implement Windows sandbox policy. Those remain later R4 integration slices.
