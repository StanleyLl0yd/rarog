# ADR-0049 — Deterministic WebIDL binding metadata

Status: Accepted

## Context

The normalized WebIDL frontend can now parse a measured standards-oriented subset into Rarog-owned IR. Generated DOM/Web API bindings need a stable semantic input rather than raw source-order fragments: partial definitions must be combined with their primary definition, includes relations must be checked, and ambiguous named definitions must fail before code generation.

This layer must remain independent of SpiderMonkey and must not pretend to implement every WebIDL validation rule before the corresponding IR and binding semantics exist.

## Decision

Add a Rarog-owned `BindingMetadata` build step over `WebIdlModule`.

`build_binding_metadata` uses ordered maps/sets so canonical output is deterministic for the same normalized definitions regardless of top-level ordering of distinct names. Named definitions are emitted in identifier order; includes relations are emitted afterwards in `(target, mixin)` order.

The first validation/canonicalization slice:

- requires at most one non-partial interface, interface-mixin or dictionary definition for each name;
- merges partial interface/interface-mixin and dictionary members into the corresponding primary definition, with primary members first and partial fragments in normalized source order;
- rejects orphan partial definitions;
- rejects conflicting named definition kinds, including interface versus interface-mixin conflicts;
- rejects inheritance on partial definitions and on interface mixins, including IR assembled directly by callers rather than only parser-produced IR;
- rejects duplicate includes relations;
- requires an includes target to resolve to a non-mixin interface and its source to resolve to an interface mixin;
- reports semantic failures through the new Rarog-owned `WebIdlErrorKind::Validation` value.

The resulting metadata continues to use Rarog-owned `Definition` values. No parser AST, code-generator runtime type or JavaScript-engine type enters the metadata API.

## Consequences

- Later binding generation gets a canonical, validated input instead of needing to rediscover partial/include relationships.
- Stable ordering does not depend on hash-map iteration.
- Parser replacement remains irrelevant to binding consumers once normalization has completed.
- This is intentionally not complete WebIDL semantic validation. Operation overloading, detailed member-name constraints, inheritance-cycle/type-resolution rules, extended attributes and other unsupported grammar remain later measured slices.
- SpiderMonkey remains outside the WebIDL crate and outside this metadata layer.
