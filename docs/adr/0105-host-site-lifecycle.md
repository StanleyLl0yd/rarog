# ADR-0105: Host control plane owns Site lifecycle authority

Status: accepted

## Context

The R4 process topology, IPC protocol and capability broker are deliberately independent primitives. Production process isolation needs one Host-owned lifecycle that composes them without letting Site-controlled code mutate topology, rebind channels or grant authority.

Crash recovery is especially sensitive to ordering. Reusing a channel, process identity or capability after a Site-process loss would let stale or delayed work regain authority in a replacement process.

## Decision

Rarog introduces `rarog-host`, depending only on the portable R4 process, IPC, broker and URL boundaries.

The Host control plane owns:

- the `ProcessTopology`;
- one bounded `IpcChannel` for each live logical `SiteProcessId`;
- the `CapabilityBroker`;
- the association between each live process identity and its `SiteIdentity`.

Same-site reuse and cross-site separation remain delegated to the Host-owned `ProcessTopology`.

Transport-facing enqueue/dequeue operations take the Host-bound `SiteProcessId` separately from the envelope. Host→Site and Site→Host direction is validated before a message reaches a channel.

Capabilities may be granted or authorized only for a currently live Site-process identity.

When a Site process is reported lost, the control plane performs these steps before replacement:

1. remove the live Site instance from Host routing;
2. disconnect its channel and discard queued work;
3. revoke every capability owned by that Site-process identity;
4. retire the process/site mapping;
5. only then permit recovery to allocate a fresh process identity and channel.

Retired process IDs are rejected by all Host routing and capability-grant entry points. Capability IDs remain invalid after process-scoped revocation even when the same site is recovered.

## Consequences

The later Windows launcher and IPC transport have one portable lifecycle contract to realize.

OS PID/handle identity does not become Web security identity.

A lost Site process cannot leave stale queued IPC or brokered authority attached to a replacement process.

The first implementation handles an explicit process-loss notification; automatic OS process-loss detection remains the responsibility of the later platform/process adapter.

Navigation integration and privileged-operation routing remain later R4 work.
