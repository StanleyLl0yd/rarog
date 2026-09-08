# ADR-0108: Host-driven engine document navigation transactions

Status: accepted

## Context

Rarog already owns canonical URL/origin primitives and the Fetch request/response model, while R4 routes privileged Network operations through Host-owned process and capability authority. The original R0 `View::navigate` API is only a policy/event forwarding seam, so an embedder that wants to complete a document navigation would otherwise have to perform HTTP and then inject decoded text with `View::load_html`.

That workaround would move Web navigation semantics into the embedder, duplicate navigation identities, and create a path around Rarog-owned Fetch and Host capability boundaries. It would also make supersession and stale network completion depend on browser-product state rather than the engine transaction that owns the document commit.

The current HTML parser accepts Unicode text, not encoded response bytes, and the current Fetch foundation does not yet implement redirect following or HTML encoding sniffing. Those missing Web semantics must remain explicit rather than being guessed by the transport or embedder.

## Decision

`rarog-engine` owns a monotonically allocated navigation identity for each live `View`. A navigation identity is scoped to that View and is never reused during the View lifetime.

`View::begin_navigation` applies `HostPolicy`, parses and canonicalizes an HTTP(S) target through `rarog-url`, derives the request origin from Rarog-owned document environment state, and constructs the high-level document `FetchRequest` inside the engine. The initial local environment receives one stable opaque origin when network navigation first requires an origin. A later committed network document replaces that environment origin with the final canonical URL origin. The opaque display `BaseUrl` accepted by local `load_html` is not promoted into a Web security identity.

The engine configures document navigation as `RequestMode::Navigate`, `CredentialsMode::Include`, `RedirectMode::Manual` and document destination. It exposes only the resulting bounded `NetworkRequest` transport projection together with the Rarog navigation identity. It does not receive or expose `NetworkCapability`, backend `NetworkTicket`, R4 `NetworkOperationId`, `SiteProcessId` or `CapabilityId`.

The embedder/Host maps that navigation identity to its authorized Host-owned network operation. When a newer valid navigation begins, the engine returns the superseded navigation identity and rejects any later completion for it. Explicit cancellation removes the pending transaction and likewise makes later completion stale. A View owner can cancel its pending transaction before destroying the View and then cancel the mapped Host operation.

A matching `FetchResponse` is consumed only by the current transaction. The response must identify the transport response URL, and that URL must exactly match the fragment-free URL projected in the current `NetworkRequest`; a missing or different URL is rejected because it could hide redirect/final-URL policy below Rarog. For a successful non-redirect navigation, the committed document URL restores the fragment from the canonical requested navigation URL while the transport comparison remains fragment-free. Redirect responses are explicit failures until Rarog-owned redirect processing issues the next transport request.

The first completion slice commits only bounded `text/html; charset=utf-8` responses whose bodies are valid UTF-8, with an optional UTF-8 BOM. Informational responses, 204/205 no-document responses, redirects, missing/unsupported media types, missing/non-UTF-8 charsets, invalid UTF-8 and oversized document bodies fail explicitly without replacing the current document. Full HTML byte encoding sniffing and redirect processing extend this engine-owned boundary later.

Navigation lifecycle changes are observable through UI-neutral `ViewEvent` values. These events carry Rarog identities and canonical URLs but no platform UI type or privileged transport authority.

## Consequences

Zorya and other embedders can drive an authorized network backend without owning URL/origin/Fetch document policy or being able to commit a stale navigation. R4 capability authorization remains mandatory because the engine exports a transport request rather than a direct network capability path.

Transport implementations must perform one exchange and surface redirects; automatic redirect following is outside the `NetworkCapability` contract.

The legacy `View::navigate` forwarding seam remains available for compatibility, while new document-loading integrations use the transaction API.

This ADR does not claim complete Fetch navigation, redirect, MIME, HTML encoding, cookie, mixed-content or subresource support. Those remain Rarog-owned follow-up work.
