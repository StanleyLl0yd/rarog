# R2 — Flight backlog

Status: **in progress**.

Tracking issue: #79.

## A — WebIDL and binding metadata

- [x] Introduce a Rarog-owned normalized WebIDL IR and parser frontend boundary.
- [x] Connect a standards-oriented WebIDL parser behind that boundary without exposing dependency AST types.
- [x] Add deterministic validation and linking across related WebIDL definitions.
- [x] Add binding metadata suitable for generated DOM/Web API bindings.

## B — Script runtime boundary

- [x] Introduce a replaceable Rarog Script API with no runtime-specific types in DOM/Web API crates.
- [x] Define value, exception, rooting and realm ownership contracts before broad bindings.
- [x] Add the first SpiderMonkey adapter behind the Script API.

## C — Events and scheduling

- [x] Add Event and EventTarget foundations.
- [x] Add engine-owned task and microtask scheduling boundaries.
- [x] Connect script-driven DOM mutations to existing invalidation and retained rendering.

## D — Fetch and identity primitives

- [x] Add URL parsing and serialization primitives.
- [x] Add origin and site identity primitives.
- [x] Add Fetch request/response foundations behind embedder network-capability boundaries.

## E — Windows host adapters

- [x] Add platform-neutral input, text-input and clipboard contracts.
- [x] Add Windows keyboard and mouse input adapter.
- [x] Add Windows IME and text-input adapter.
- [ ] Add Windows clipboard adapter.

## Scope boundary

Flexbox/grid, GPU/compositor, asynchronous image decode and scrolling remain R3. Process isolation work remains R4. Broader storage, media, workers and accessibility remain later milestones.
