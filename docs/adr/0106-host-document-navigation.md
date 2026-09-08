# ADR-0106: Host-owned document/Site navigation transitions

**Status:** Accepted  
**Date:** 2026-09-08

## Context

R4 already assigns schemeful sites to bounded Host-owned Site-process identities, but a document environment also needs an explicit binding to that authority. Recomputing process or opaque-site identity from untrusted navigation payloads would make replacement and stale-authority behavior ambiguous.

Opaque URLs are especially sensitive: asking `WebUrl` for an opaque origin creates a fresh Rarog identity. That is correct when creating a new environment and incorrect when code intends to refer to an existing inherited environment.

## Decision

`rarog-host` owns `DocumentSiteBinding`, which stores the environment's `SiteIdentity` and Host-assigned `SiteProcessId`.

Navigation transitions obey these rules:

- initial document navigation derives one Site identity and obtains a bounded Host assignment;
- same-site navigation reuses the current Site process;
- cross-site and cross-scheme navigation replace the document binding with a distinct Site process;
- a new opaque navigation derives a fresh opaque Site identity exactly once;
- inherited opaque environments pass their already-owned `SiteIdentity` explicitly;
- the current document binding is validated before target assignment, so stale process authority fails closed;
- process-budget failure leaves the current binding valid and never falls back to cross-site sharing.

A transition does not automatically retire the previous Site process because other document environments may still be bound to the same site. Process lifetime remains Host-owned and is retired by explicit lifecycle/process-loss handling.

## Consequences

Document/Site authority is explicit and testable without exposing OS process identifiers or Windows handles to portable code. The later Windows launcher and IPC transport can bind concrete processes/endpoints to the same Host-owned identity contract.

The engine's R0 `View::navigate` contract remains a policy/event boundary that forwards navigation to the embedder. R4 Host code owns Site-process assignment after that boundary; Web content never selects a process identity directly.
