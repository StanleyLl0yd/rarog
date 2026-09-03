# ADR-0051 — Script value, exception and rooting ownership

Status: Accepted

## Context

ADR-0050 establishes the replaceable `rarog-script` realm lifecycle/evaluation boundary without committing Rarog to SpiderMonkey types. The next R2 slice must define how JavaScript values can survive an adapter call before generated WebIDL bindings, DOM wrappers, event listeners and queued callbacks begin retaining script-visible state.

SpiderMonkey values and objects may move or become unreachable under garbage collection. Rarog therefore cannot expose raw backend values, object pointers or unrooted handles across the adapter boundary. JavaScript exceptions also need to remain distinct from runtime/adapter failures: a script `throw` is normal JavaScript completion semantics, not an infrastructure error.

## Decision

Any JavaScript value that crosses the `ScriptRuntime` boundary and remains observable after the adapter call is represented by an opaque Rarog-owned `RootedValueId`.

A rooted-value identity is scoped to a `RealmId`. It is not a pointer, object identity, serialized value or backend GC handle. A runtime adapter owns the mapping from each live `RootedValueId` to the concrete backend root and must reject foreign, stale or already released handles.

`duplicate_root` creates an independently releasable root for the same backend value. Copying a `RootedValueId` only copies the identifier and does not create another ownership unit. `release_root` invalidates that root identity. Destroying a realm releases every remaining root owned by the realm and invalidates all of its rooted-value identities.

Every realm has one `GlobalObjectId` with realm lifetime. The global is distinct from ordinary rooted-value identities so callers cannot release the realm global through `release_root`. `ScriptRealm` returns both the realm identity and its global identity from realm creation.

`ScriptRealmLimits` makes source bytes and persistent rooted values explicit per-realm resource limits. Concrete adapters must enforce the source limit before backend compilation/evaluation and the rooted-value limit before creating another persistent root. The mandatory realm-global root is part of realm lifetime rather than the releasable rooted-value budget.

Evaluation returns a `ScriptCompletion`:

- `Normal(RootedValueId)` for normal JavaScript completion;
- `Throw(ScriptException)` for a JavaScript exception.

`ScriptException` owns normalized message/stack text when available and owns a rooted handle for the thrown JavaScript value. Backend initialization failures, invalid realm/root handles, exhausted limits and adapter failures continue to use `ScriptError`. A JavaScript `throw` must not be converted into `ScriptError::Backend` merely to simplify an adapter.

The Rarog boundary does not yet define primitive conversion, property access, calls, constructors, promises or DOM wrapper behavior. Those operations will consume and produce rooted handles behind the same ownership rules.

## Consequences

- No unrooted backend JS value may escape a `ScriptRuntime` method.
- Root liveness is explicit and independently testable before SpiderMonkey is connected.
- Event listeners, task queues and DOM wrapper registries can later retain script values by owning roots without holding SpiderMonkey pointers.
- Realm teardown provides a hard lifetime boundary that clears leaked or abandoned roots before native document storage is released, preserving ADR-0013 teardown ordering.
- JavaScript exceptions remain Web semantics while `ScriptError` remains a runtime-contract failure channel.
- Resource limits bound persistent script roots independently of backend GC heuristics.
