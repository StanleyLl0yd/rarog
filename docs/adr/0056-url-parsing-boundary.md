# ADR-0056: Rarog-owned URL parsing boundary

Status: accepted

## Context

R2 fetch, navigation, origin and site policy need standards-oriented URL parsing, canonical serialization and relative-reference resolution. URL semantics are security-sensitive and should not be reimplemented ad hoc inside networking, DOM or embedder code.

The rest of Rarog must also remain independent of a specific parser crate. Public APIs should not expose `url::Url`, `url::Host` or `url::ParseError`, because doing so would make parser replacement and dependency upgrades part of the engine-wide API surface.

## Decision

Rarog owns URL primitives in the dedicated `rarog-url` crate.

The first implementation uses the `url` crate pinned to 2.5.7. That implementation is based on the WHATWG URL Standard and declares Rust 1.63 as its minimum supported Rust version, below Rarog's Rust 1.85 workspace floor.

`WebUrl` privately owns the parser representation and exposes only Rarog-owned values. Parsing and relative resolution return `UrlError`, which maps dependency parser failures into `UrlErrorKind` plus an owned diagnostic message. Hosts are converted into the Rarog-owned `UrlHost` enum for domain, IPv4 and IPv6 identities.

The boundary exposes canonical serialization and the URL components needed by later navigation, request and origin work: scheme, credentials, host, explicit/effective port, path, query and fragment. Fragment stripping returns an independent URL value suitable for later request construction without mutating the original URL.

Dependency parse errors are non-exhaustive, so unknown future parser errors map to `UrlErrorKind::Other` rather than forcing a public Rarog API change.

## Consequences

DOM, engine, fetch and policy code can depend on stable Rarog URL semantics without importing parser-specific types. IDNA handling, special-scheme normalization, default-port normalization and relative resolution come from a mature standards-oriented parser instead of custom code.

Origin and site identity are deliberately a following slice in the same Rarog-owned boundary. Existing embedder `BaseUrl` remains unchanged until navigation/resource-request migration can be done deliberately rather than coupling this parser foundation to an unrelated public API change.

Network I/O, DNS resolution, origin policy, public-suffix/site calculation and Fetch request behavior are out of scope for this decision.
