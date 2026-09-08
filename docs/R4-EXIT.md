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

The completed R4 foundation slices establish process/site identities, bounded Host/Site IPC, Host-owned capability authorization, explicit document-to-Site navigation bindings, bounded Host-owned navigation contexts with last-reference Site lifetime, context-scoped privileged authority, broker-gated Network/Clipboard routing, and Windows child-process launch/loss observation that feeds Host revocation/retirement. They do not claim that production Site execution is already moved into the launched child or that the Windows sandbox/mitigation policy is complete yet.

## Required automated gate

Before R4 can close, a dedicated `crates/rarog-engine/tests/r4_exit.rs` gate must verify at minimum:

- the R4 backlog has no unchecked milestone items;
- distinct schemeful sites cannot share Site-process authority;
- same-site reuse is explicit and bounded;
- cross-site/cross-scheme navigation replaces document bindings without cross-site process sharing;
- Host navigation-context identities are bounded, monotonic and not reused after close/invalidation;
- same-site contexts can share a Site process while last-reference close/transition retires unreferenced source authority;
- a one-slot cross-site replacement retires only an unshared source and explicitly invalidates the context if target allocation then fails;
- opaque Site identity is explicitly propagated by the owning document environment rather than recomputed;
- process-budget exhaustion fails closed;
- IPC version/size/role validation and backpressure behavior;
- the fixed wire codec rejects malformed/truncated/trailing/oversized frames before unbounded payload allocation;
- Windows IPC transport preserves a separately supplied Host-owned Site-process binding and rejects forged envelope direction;
- capability ownership, revocation and stale-process rejection;
- production navigation-context Network/Clipboard routes require exact Host context/process/capability authorization before reaching a backend;
- lower-level process-owned privileged routes remain authenticated Host/transport building blocks rather than Site-selected authority;
- two contexts sharing one Site process cannot interchange context-scoped capability authority;
- backend Network tickets are hidden behind bounded Host-owned operation identities and stale operation authority is rejected;
- real Windows child-process loss invokes Host revocation/retirement once and produces fresh replacement authority;
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
