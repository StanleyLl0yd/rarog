# ADR-0050 — Replaceable script runtime boundary

Status: Accepted

## Context

R2 needs JavaScript execution for DOM and Web API bindings, but SpiderMonkey must remain replaceable and its native value, object, rooting and realm types must not leak through engine crates. The first script-facing slice therefore has to establish lifecycle and evaluation ownership before broad bindings or a concrete runtime adapter are introduced.

ADR-0002 selects SpiderMonkey as the first production JavaScript/Wasm backend without making it part of Rarog's public engine architecture. ADR-0013 also requires DOM wrappers to resolve through document/realm ownership rather than raw DOM arena pointers and fixes teardown ordering before wrapper integration begins.

## Decision

Introduce `rarog-script` as the engine-owned script-runtime boundary.

The crate exposes an object-safe `ScriptRuntime` contract with three initial operations:

- create an opaque realm identity;
- evaluate borrowed script source in a live realm;
- destroy a realm so later operations using that identity fail safely.

`RealmId` is an opaque Rarog type. Its numeric representation is private and is meaningful only together with the runtime instance that created it. Runtime implementations must validate realm liveness and must not treat the identifier as a native pointer or expose backend realm handles through it.

`ScriptSource` borrows source text and an optional source name for the duration of one evaluation call. The boundary does not retain source buffers implicitly. Callers can apply an explicit byte limit before backend work with `ensure_byte_limit`; the concrete engine integration remains responsible for deriving that limit from the applicable resource budget.

Diagnostics and errors crossing the boundary are Rarog-owned values. Messages, source names and diagnostic metadata are owned so no parser/runtime buffer lifetime escapes an evaluation call. Backend-specific error objects must be normalized before leaving an adapter.

The initial evaluation result intentionally contains diagnostics only. JavaScript values, objects, exceptions as values, rooting/tracing handles, globals and wrapper registries are not guessed into this first API. Those ownership contracts are the next R2 script slice and must be defined before broad generated DOM bindings.

The contract does not require `Send` or otherwise encode a threading model. A SpiderMonkey adapter may remain thread-affine while the portable engine boundary stays independent of SpiderMonkey execution details.

## Consequences

- DOM, WebIDL, engine and platform crates do not need SpiderMonkey types to refer to script realms or request evaluation.
- A SpiderMonkey adapter can be added as a separate crate behind `ScriptRuntime`.
- Realm destruction and invalid-realm behavior are explicit before event queues and DOM wrappers can retain realm-owned state.
- Script source remains borrowed and can be resource-checked before entering a backend.
- Runtime values, rooting/tracing and wrapper identity remain deliberate follow-up work rather than accidental API commitments.
- Document/realm shutdown must continue to follow the ordering fixed by ADR-0013 when script execution is connected to `RenderSession`.
