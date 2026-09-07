# R4 — Sky exit gate

Status: **in progress**.

## Purpose

R4 establishes the first process-isolation architecture required by ADR-0003:

- Host and Site process ownership;
- versioned bounded IPC;
- Host-owned capability brokering;
- Windows-first Site-process sandbox/process hardening;
- site isolation by default;
- crash/replacement handling with stale-authority revocation.

The completed R4 foundation slices establish process/site identities, bounded host-side assignment, and a versioned bounded Host/Site IPC protocol with backpressure and disconnect semantics. They do not claim OS process separation, a Windows transport or sandboxing yet.

## Required automated gate

Before R4 can close, a dedicated `crates/rarog-engine/tests/r4_exit.rs` gate must verify at minimum:

- the R4 backlog has no unchecked milestone items;
- distinct schemeful sites cannot share Site-process authority;
- same-site reuse is explicit and bounded;
- process-budget exhaustion fails closed;
- IPC version/size/role validation and backpressure behavior;
- capability ownership, revocation and stale-process rejection;
- Site-process loss produces fresh replacement authority;
- the Windows process/sandbox adapter passes its platform-specific contract.

The full workspace suite remains responsible for narrower protocol, process, broker, navigation and platform regressions.

## Architecture invariants

R4 must preserve:

- Web security identity is owned by Rarog URL/origin/site primitives, not by OS PID, transport connection or backend objects;
- Host code is the authority for process assignment and privileged capabilities;
- Site processes cannot mint or widen Host-owned capability authority;
- process, channel and capability identities are not reused after retirement;
- distinct sites are never colocated merely to satisfy a memory/process budget;
- Windows-specific launch/sandbox APIs remain behind platform boundaries;
- all Web-controlled protocol payloads and queues are explicitly bounded;
- process loss fails closed before recovery;
- Rust 1.85 MSRV and existing deterministic rendering/security gates remain intact.

## Explicit deferrals

R4 does not include R5 storage/workers/media/accessibility, R6 compatibility qualification, the stable R7 embedding ABI or Zorya browser UI.

R4 also does not claim that the first bounded Windows sandbox policy is equivalent to Chromium's mature sandbox or that all future Web-platform process types are already represented.
