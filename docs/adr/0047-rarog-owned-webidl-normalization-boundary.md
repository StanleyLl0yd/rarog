# ADR-0047 — Rarog-owned WebIDL normalization boundary

Status: Accepted

## Context

R2 — Flight introduces WebIDL and later script bindings. The architecture already requires script runtimes and parser dependencies to remain replaceable. Allowing a third-party WebIDL AST to become the public binding model would couple DOM/Web API work to one parser's lifetimes, enums and grammar-version choices.

## Decision

Introduce `rarog-webidl` before selecting the concrete parser adapter.

The crate owns:

- owned `Identifier` values;
- normalized interface, dictionary, enum, typedef and includes definitions;
- normalized attributes, operations and arguments;
- normalized WebIDL type shapes;
- source diagnostics/errors;
- deterministic module snapshots;
- the object-safe `WebIdlFrontend` parser boundary.

Parser adapters must convert dependency AST values into these owned types before returning to callers. No dependency AST type may appear in a public `rarog-webidl` signature or in downstream DOM/script crates.

The first slice intentionally contains no parser dependency. A following slice may select and pin a standards-oriented WebIDL parser only after Windows, Linux and Rust 1.85 compatibility are proven in CI.

## Consequences

- Parser replacement remains local to `rarog-webidl`.
- Downstream binding generation can evolve against stable engine-owned semantics.
- Source text and parser-borrowed lifetimes do not leak into long-lived binding metadata.
- The initial IR is deliberately smaller than complete WebIDL. Unsupported constructs must fail or diagnose explicitly until normalized support is added; they must not be silently discarded.
- SpiderMonkey integration remains a later Script API adapter concern and is not a dependency of this crate.
