# ADR-0057: Origin and schemeful-site identity

Status: accepted

## Context

R2 needs security identity primitives before Fetch request/response policy can be represented correctly. URL equality is not an origin check, and host equality is not a same-site check. Origin identity depends on scheme, host and effective port for tuple origins, while schemeful-site identity groups domain hosts at the registrable-domain boundary and still keeps the scheme significant.

The URL implementation dependency must not become the identity model exposed to DOM, script, engine or embedder crates. Opaque origins also need Rarog-owned identity rather than backend-owned opaque tokens whose representation and lifecycle are implementation details.

## Decision

`rarog-url` owns `Origin`, `OpaqueOriginId` and `SiteIdentity`.

Tuple origins store normalized Rarog-owned scheme, `UrlHost` and effective port values. Opaque origins allocate process-unique non-zero Rarog identities. Copying or cloning an opaque identity preserves equality; independently derived opaque origins remain unequal. ASCII serialization of an opaque origin is `null`.

Origin derivation uses the pinned `url` implementation internally so special URL and `blob:` origin behavior follows the same standards-oriented parser used by `WebUrl`, but no `url::Origin` or `url::Host` type crosses the crate boundary.

Schemeful sites are derived from origins. The port is intentionally excluded. Domain hosts are reduced to their registrable domain using a pinned snapshot of Mozilla's Public Suffix List through `psl` 2.1.226. If a domain has no registrable-domain result, its normalized exact host is retained. IPv4 and IPv6 hosts remain exact. Opaque sites preserve the originating opaque identity.

The Public Suffix List dependency is pinned exactly because list changes can change security identity. A PSL update therefore requires an explicit Rarog dependency change and normal CI review rather than silently altering same-site behavior.

## Consequences

Rarog can distinguish same-origin from same-site before networking is connected. Ports separate origins but not sites; schemes separate both schemeful sites and tuple origins; subdomains under one registrable domain share a site; private suffixes represented by the pinned PSL remain isolation boundaries.

Environment-origin inheritance rules such as initial `about:blank`, sandboxed documents, workers and broader agent-cluster policy are not encoded in `WebUrl::origin()`. Those contexts will own and propagate an `Origin` explicitly when their lifecycle is introduced rather than repeatedly deriving identity from a URL.

This slice does not implement CORS, credentials modes, cookie policy, storage partition keys, Fetch tainting, mixed-content checks or network transport. Those consume these primitives in later boundaries.
