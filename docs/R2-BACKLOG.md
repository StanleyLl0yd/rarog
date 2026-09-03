# R2 — Flight backlog

Status: **in progress**.

Tracking issue: #79.

## A — WebIDL and binding metadata

- [x] Introduce a Rarog-owned normalized WebIDL IR and parser frontend boundary.
- [ ] Connect a standards-oriented WebIDL parser behind that boundary without exposing dependency AST types.
- [ ] Add deterministic validation and linking across related WebIDL definitions.
- [ ] Add binding metadata suitable for generated DOM/Web API bindings.

## B — Script runtime boundary

- [ ] Introduce a replaceable Rarog Script API with no runtime-specific types in DOM/Web API crates.
- [ ] Define value, exception, rooting and realm ownership contracts before broad bindings.
- [ ] Add the first SpiderMonkey adapter behind the Script API.

## C — Events and scheduling

- [ ] Add Event and EventTarget foundations.
- [ ] Add engine-owned task and microtask scheduling boundaries.
- [ ] Connect script-driven DOM mutations to existing invalidation and retained rendering.

## D — Fetch and identity primitives

- [ ] Add URL parsing and serialization primitives.
- [ ] Add origin and site identity primitives.
- [ ] Add Fetch request/response foundations behind embedder network-capability boundaries.

## E — Windows host adapters

- [ ] Add platform-neutral input, text-input and clipboard contracts.
- [ ] Add Windows keyboard and mouse input adapter.
- [ ] Add Windows IME and text-input adapter.
- [ ] Add Windows clipboard adapter.

## Scope boundary

Flexbox/grid, GPU/compositor, asynchronous image decode and scrolling remain R3. Process isolation work remains R4. Broader storage, media, workers and accessibility remain later milestones.
