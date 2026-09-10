# ADR-0122: Service Worker fetch interception remains Host-authorized

- Status: accepted
- Date: 2026-09-10
- R5 tracking: #260, #279

## Context

R5 now has bounded Service Worker registration/scope/version state, but those identities are intentionally references rather than network authority. Rarog already has a Host-owned navigation-context model, an exact Site-process/capability broker boundary, bounded Fetch request/response types, and Host-owned network-operation identities whose backend tickets are private. Service Worker interception must compose these authorities without allowing a Site, registration ID, version ID, controller ID, or dispatch ID to create a parallel route to the network backend.

A navigation-context Network capability alone is insufficient for Service Worker control because same-site cross-origin navigation may retain that capability. Service Worker controller selection additionally needs the Host-authenticated current client URL and exact current origin. The Service Workers model also allows a controlled client's subresource request URL to be cross-origin, so request URL origin cannot replace client-origin authority.

## Decision

`HostControlPlane` retains a bounded, fragmentless current `WebUrl` for URL-backed navigation contexts. Site-only navigation contexts have no current URL and are ineligible for this interception foundation. Navigation clears controller and pending dispatch state before a replacement controller can be selected. Controller selection is performed only by the Host by matching that retained current URL against `ServiceWorkerRegistry`; callers cannot nominate a registration or version as authority.

A `ServiceWorkerController` records the registration/version selected by the Host. It remains a reference: every fetch-policy access revalidates that the navigation context still has the same non-opaque exact origin, the registration still belongs to that origin, the registration active slot still names the exact version, and the version is `Activating` or `Activated`. A stale or replaced controller fails closed. This R5 foundation deliberately requires an explicit Host refresh after navigation or controller invalidation rather than claiming complete Service Worker client/controller handoff semantics.

`prepare_navigation_context_service_worker_fetch` first validates the exact live navigation-context Network capability and requires `FetchRequest.origin` to equal the Host-owned current client origin. A controlled cross-origin subresource URL is therefore allowed without allowing the request to forge the client origin. An uncontrolled request returns an owned `NetworkFallback(FetchRequest)`. An active worker still in `Activating` returns an owned `AwaitingActivation { controller, request }`; it does not touch the backend. Only an `Activated` controller can allocate a bounded Host-owned `ServiceWorkerFetchDispatchId` and retain the exact bounded `FetchRequest` in the pending dispatch table.

Dispatch identities are scoped monotonic references, not capabilities. Reading, discarding, or completing a dispatch requires the exact current navigation-context Network capability, exact Host process/context binding, and a still-valid activated controller. Navigation, capability revocation, context close, Site-process loss and explicit stale-dispatch reaping remove dispatch authority and recover bounded capacity.

A Service Worker completion can return a bounded `FetchResponse` or explicit fallback. Response limits are revalidated against the originating request before the dispatch is released; an invalid response leaves the dispatch charged until valid completion or explicit discard. Explicit fallback consumes the retained request and enters the existing Host-authorized `NetworkOperationId` / `NetworkCapability` path. Backend `NetworkTicket` values never enter the Service Worker API surface.

## Consequences

This creates no new backend/network authority class. Service Worker registration/version/controller/dispatch identities cannot directly start, poll or cancel a network backend operation. The existing Host/network capability path remains the sole transport authority and retains its existing backpressure, revocation and backend-ticket quarantine behavior.

The current R5 boundary is intentionally narrower than complete Service Worker Fetch semantics. It does not expose script-visible `FetchEvent`/`respondWith`, navigation interception completeness, Cache Storage, navigation preload, router rules, soft-update timing, full storage-key partitioning, trustworthy-origin/secure-context completeness, or complete client/controller transfer semantics. Those omissions must not be inferred as implemented compatibility evidence.
