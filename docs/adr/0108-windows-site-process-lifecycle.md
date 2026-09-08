# ADR-0108: Windows Site-process launch and loss adapter

**Status:** Accepted  
**Date:** 2026-09-08

## Context

The portable R4 Host control plane already knows how to invalidate a lost logical Site process, but it had no concrete Windows child lifetime feeding that transition. Treating an OS PID or process handle as Rarog security identity would couple portable authority to platform objects and make stale-process behavior ambiguous.

This slice must also avoid claiming a sandbox before the mitigation policy exists.

## Decision

`rarog-platform-windows` owns a narrow `WindowsSiteProcess` adapter.

- Launch accepts a live Host-produced `SiteLease`, not a Site-controlled numeric process identity.
- The adapter verifies the lease is still live before spawning.
- `std::process::Child`, its PID and Windows process details remain private to the target-specific module.
- Standard input, output and error are redirected to null; Windows launch uses the no-console creation flag.
- Command argument count is explicitly bounded.
- Non-blocking status observation, blocking wait and explicit termination all report child loss through `HostControlPlane::process_lost` once.
- Successful loss reporting therefore inherits the Host's disconnect, network-operation revocation, capability revocation and process retirement ordering.
- Repeated observation after delivery returns no second loss.
- A still-live adapter dropped without its Host is killed and reaped as emergency containment. Normal recovery uses an explicit reporting method so logical authority is invalidated before replacement.
- Non-Windows builds expose explicit unsupported-target behavior and contain no Windows handle representation.

The implementation uses safe standard-library process APIs. This ADR does not define restricted tokens, job objects, AppContainer policy, process mitigation attributes or the Site sandbox.

## Consequences

A real Windows child exit now reaches the existing R4 fail-closed logical lifecycle without making OS process identifiers authoritative. Windows-primary CI launches a short-lived child, proves capability and derived network-operation authority are revoked, and proves recovery receives a fresh Site-process identity.

The later sandbox/mitigation slice may replace the concrete spawn mechanics with a narrower audited Windows-native launcher while preserving the same Host-owned `SiteLease` and loss-reporting contract.
