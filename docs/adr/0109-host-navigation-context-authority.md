# ADR-0109: Host-owned navigation contexts and context-scoped authority

**Status:** Accepted  
**Date:** 2026-09-08

## Context

ADR-0106 established explicit `DocumentSiteBinding` values and same-site/cross-site Site-process transition rules, while ADR-0107 established process-owned capability checks and Host-owned Network operation identities. A production embedder still needs a stable document/navigation lifetime handle without becoming authoritative for `SiteProcessId`.

Process ownership alone is also too coarse for browser-style multi-context reuse: two same-site documents may intentionally share one Site process, but that must not make a capability issued for one document usable by the other merely because the broker sees the same process owner.

## Decision

`rarog-host` owns bounded monotonic `NavigationContextId` identities. A live context owns one current `DocumentSiteBinding`; public context snapshots expose the context and Site identity while process selection remains Host state.

Context lifecycle obeys these rules:

- context identities are non-zero, bounded by a configured live-context limit, monotonically allocated and never reused after close or invalidation;
- same-site navigation retains the current Site-process binding;
- cross-site navigation moves the context to the target Site process and revokes capabilities scoped to the moving context before stale authority can be reused;
- Site-process lifetime is reference-counted by live Host contexts; a source process is retired only after its last context leaves;
- when a one-slot process budget blocks an unshared cross-site move, the Host may retire the source before target allocation rather than weakening isolation;
- if that post-retirement target allocation fails, the context is explicitly invalidated; the old process/capability authority is not restored or shared;
- explicit process loss invalidates every bound context before recovery can issue fresh authority;
- inherited opaque environments continue to pass their exact stored `SiteIdentity`; URL text is not used to regenerate an existing opaque identity.

`NavigationContextCapability` binds a broker-issued capability to the exact Host context as well as the broker's Site-process owner and class. Context-scoped Network/Clipboard entry points resolve the current process from Host context state, verify context ownership, then delegate to the existing process/capability/class checks. A numeric capability reference from another same-process context therefore remains insufficient authority.

The lower-level process-owned Host routes remain available as portable control-plane building blocks for authenticated Host/transport code. Embedders should use context-owned authority when representing document/navigation lifetime.

## Consequences

Zorya and other embedders can map engine navigation transactions to Host context/capability handles without supplying authoritative Site-process identities. Same-site process reuse no longer broadens document-level privilege scope, and close, cross-site transition or process loss invalidates stale context authority deterministically.

This decision does not add a Windows IPC transport, create OS processes, implement sandbox tokens/job objects, detect process loss automatically, or move Fetch/Web policy into `rarog-host`. Those remain separate R4 or later integration work.
