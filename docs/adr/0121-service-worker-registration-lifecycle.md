# ADR-0121: Bounded Service Worker registration, scope and lifecycle foundation

- Status: Accepted
- Date: 2026-09-10

## Context

R5 already owns bounded dedicated-Worker identity, execution and message transport. Service Workers additionally need persistent registration identity, canonical scope ownership and a version lifecycle before Fetch interception can be authorized safely. That state must remain portable and Rarog-owned: Web input must not select Host/Site/Storage process authority, OS objects, backend tickets or SpiderMonkey objects.

The current Service Workers specification keys registrations by storage key plus serialized scope URL and defines installing, waiting and active worker slots. Rarog does not yet have the later client/storage-key integration required to claim that complete model, so this slice must establish a narrower exact-origin foundation without misrepresenting it as full Service Worker API or partitioning support.

## Decision

`rarog-workers` owns `ServiceWorkerRegistry`, scoped monotonic `ServiceWorkerRegistrationId` and `ServiceWorkerVersionId` references, canonical registration scope state and explicit installing/waiting/active version slots. IDs are correlation references only and do not grant Fetch, Host, process or platform authority.

Every registration in this foundation is bound to one exact non-opaque Rarog `Origin` and one canonical Rarog `WebUrl` scope. Scope and script URLs must be same-origin HTTP(S), reject ASCII-case-insensitive `%2f` or `%5c` sequences in their URL paths, and are canonicalized without fragments through a bounded serialized prefix so an attacker-controlled fragment is neither retained nor copied before the canonical URL-byte budget is enforced. Global registration count, per-origin registration count, live version count and per-URL retained bytes are explicitly bounded; failed validation or capacity checks leave existing registration state unchanged.

Exact canonical scope identifies one registration inside the current exact-origin registry model. Re-registering that scope creates a fresh installing version and replaces only a superseded installing attempt. An existing waiting version remains intact while the new install is pending; only successful installation retires that previous waiting identity and moves the exact installing version to waiting. An already-active version remains intact until activation begins.

Activation follows the registration-slot semantics rather than treating the activate event as a transaction around the old worker. `begin_activate` first validates the waiting version and any current active slot; if that active version is still `Activating`, replacement activation fails closed without changing either slot. Once the active version is eligible for replacement, `begin_activate` retires it, promotes the waiting version into the active slot and marks it `Activating`. The retired version identity immediately becomes stale. `finish_activate` only advances that same active version to `Activated`; it does not resurrect or retain the previous active version.

Matching is deliberately data-only. `match_registration` requires the exact same Rarog origin and chooses the longest serialized scope string that prefixes the canonical client URL. A match returns only a registration reference; it does not authorize request interception or backend access.

Explicit version discard and registration unregister remove retained version state and recover bounded capacity deterministically. Foreign-registry, retired and otherwise stale registration/version references fail closed because registry scopes and serials are not reused during the registry lifetime.

## Validation

Coverage demonstrates:

- invalid limits fail before registry creation;
- opaque, cross-origin and non-HTTP(S) registration input is rejected without retained state;
- scope/script fragments are excluded from bounded canonicalization before retained URL-byte budgeting and retention;
- escaped slash/backslash path sequences are rejected case-insensitively;
- URL, registration, per-origin registration and live-version limits fail closed and capacity is recoverable after cleanup;
- exact-scope updates replace a superseded installing attempt immediately but retain the previous waiting version until the replacement install succeeds;
- activation blocks behind an already-activating active version, then promotes waiting to active/activating and retires the previous activated identity;
- invalid lifecycle transitions leave slots unchanged;
- same-origin longest-prefix scope matching is deterministic;
- unregister/discard make old IDs stale, and independent registries do not alias ordinary IDs.

## Consequences

R5 now has a bounded portable Service Worker registration and version-lifecycle authority-neutral foundation that can later be connected to client/storage-key policy and Fetch interception without making serialized URLs or numeric IDs capabilities.

This ADR does **not** claim current Service Worker specification completeness. Full storage-key partitioning is not represented by the exact-origin key used in this foundation. Secure-context/potentially-trustworthy client eligibility and script-visible registration authorization remain client/Host integration work. Response-derived maximum-scope enforcement such as `Service-Worker-Allowed` requires the later script Fetch/update path and is not inferred here. Fetch interception, request routing, script-visible Service Worker events/APIs, Cache Storage, push/sync, SharedWorker semantics, OS thread/process isolation and all R6 compatibility qualification remain out of scope.