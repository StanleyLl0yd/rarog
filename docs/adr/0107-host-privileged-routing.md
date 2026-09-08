# ADR-0107: Broker-gated privileged Network and Clipboard routing

**Status:** Accepted  
**Date:** 2026-09-08

## Context

R4 already has Host-owned process identity and capability grants, but a capability is only meaningful if concrete privileged operations cannot bypass its authorization. Network routing also has a second untrusted-reference problem: backend `NetworkTicket` values are transport implementation details and must not become cross-Site authority if guessed or confused.

Fetch policy and transport authority are separate concerns. The Host broker must gate the transport operation without becoming an alternate CORS/origin/credentials/redirect policy implementation.

## Decision

`rarog-host` routes the initial privileged Network and Clipboard classes.

For Network:

- callers provide the bounded `NetworkRequest` transport projection produced by the Fetch boundary;
- the Host validates the authenticated `SiteProcessId`, capability identity and exact Network class before calling `NetworkCapability::start`;
- backend `NetworkTicket` values remain private to the Host;
- Sites receive a bounded monotonic `NetworkOperationId` instead;
- every poll/cancel re-authorizes the capability and verifies the operation's stored process+capability ownership before touching the backend;
- completion, cancellation, capability revocation and process loss remove operation authority;
- active Host network operations have an explicit configurable limit and identities are never reused;
- if a backend reuses a still-live ticket, affected Host operation authority fails closed rather than aliasing two Site requests.

For Clipboard:

- read/write first authorize the authenticated process, capability identity and exact Clipboard class;
- text is revalidated against the concrete `PlatformClipboardService` limit before a write, and backend read text is revalidated before returning to the caller;
- authorization failures do not invoke the platform service.

The Network route does not implement or bypass Fetch policy. It accepts only the transport-level request boundary and preserves existing Fetch/backend errors.

## Consequences

A numeric capability or operation reference is never sufficient to reach a privileged backend. Network backend tickets are not security identities exposed to Site code, and stale process/capability/operation authority is removed by Host lifecycle events.

The portable Host crate now depends on the portable Fetch and Platform contracts, not on Windows implementations. Windows process/transport work can bind authenticated endpoints to this same route without placing Win32 handles or APIs in portable code.
