# ADR-0052: SpiderMonkey adapter and FFI boundary

Status: Accepted

## Context

ADR-0002 selected SpiderMonkey as the first production JavaScript/Wasm backend while requiring the engine to remain replaceable. ADR-0050 introduced the Rarog-owned `ScriptRuntime` boundary, and ADR-0051 defined realm, global, exception and rooted-value ownership before connecting a concrete collector.

SpiderMonkey's JSAPI is a C++ FFI surface with tracing-GC invariants. Raw JSAPI pointers and values cannot be allowed to escape into DOM, Web API or engine crates, and values that survive an adapter call must remain rooted for their complete Rarog-visible lifetime.

## Decision

The first backend lives in the dedicated `rarog-script-spidermonkey` crate and is enabled explicitly through its `spidermonkey` feature. The crate pins `mozjs` 0.21.6, which binds SpiderMonkey 140.14.0 from the Firefox ESR 140 line.

The adapter owns all SpiderMonkey-specific types. Its public surface exposes only `SpiderMonkeyEngine`, `SpiderMonkeyRuntime` and the Rarog-owned script contract. No JSAPI, `mozjs`, GC pointer, rooting type or backend exception type crosses into `rarog-script`, DOM or Web API crates.

`SpiderMonkeyEngine` owns process-level JS engine initialization. A `SpiderMonkeyRuntime` borrows the engine lifetime and owns the JS context. This prevents the engine owner from being dropped while a runtime remains live. Runtime teardown clears all realm state before the JS context is destroyed.

Each live realm owns:

- its Rarog `ScriptRealmLimits`;
- a persistent traced root for the SpiderMonkey global object;
- one rooted-value allocator;
- a map from opaque Rarog `RootedValueId` handles to persistent traced SpiderMonkey values.

Persistent globals and values use `RootedTraceableBox<Heap<...>>`. Stack-local evaluation results use SpiderMonkey's stack rooting before being copied into a persistent root. Releasing a Rarog root drops the persistent traced value. Destroying a realm drops every value root and the global root before native document storage may be released, preserving ADR-0013 and ADR-0051 teardown ordering.

JavaScript exceptions are completion semantics. When evaluation reports a pending exception, the adapter clears and normalizes it into `ScriptCompletion::Throw`, roots the thrown value and carries an owned message when SpiderMonkey provides one. Failure without a pending exception is a backend failure and becomes `ScriptErrorKind::Backend`.

Per-realm source and rooted-value budgets are checked at the Rarog boundary. Evaluation is rejected before script execution when there is no capacity to retain its completion value.

The adapter crate deliberately does not inherit the workspace-wide `unsafe_code = "forbid"` lint because calling JSAPI requires a narrow unsafe FFI operation. The unsafe operation is isolated at global-object creation; normal engine crates retain the workspace prohibition. `unsafe_op_in_unsafe_fn` is denied in the adapter crate.

SpiderMonkey is not built in ordinary workspace jobs. The dependency is optional and dedicated Windows/Linux CI enables the backend while using Servo's published prebuilt archives. This keeps the normal portability and Rust 1.85 checks focused on Rarog-owned code while still compiling and executing the real backend in dedicated jobs.

## Consequences

The first production backend can now execute real JavaScript without making SpiderMonkey part of Rarog's DOM or Web API type system. GC ownership and teardown remain explicit and testable through opaque handles.

The process-level SpiderMonkey engine is expected to be initialized once and retained for the browser process lifetime. Additional runtime/thread architecture is deferred until event-loop and process-isolation work requires it.

Primitive conversions, object/property/call operations, promises/microtasks, DOM wrappers and generated WebIDL bindings remain subsequent slices. Exception stack normalization may be expanded later without changing the Rarog-owned completion contract.
