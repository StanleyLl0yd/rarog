# R0 — Ember backlog

R0 has one purpose: prove the shape of the engine with a deterministic end-to-end rendering path.

## P0 — repository health

- [x] Cargo workspace and crate boundaries
- [x] workspace-wide `unsafe_code = forbid`
- [x] CI: fmt/check/clippy/test/bootstrap render
- [x] architectural decision records
- [ ] add unit tests for DOM arena invariants
- [ ] add parser fixture tests
- [ ] add golden display-list tests
- [ ] add deterministic framebuffer hash test
- [ ] add benchmark harness with no performance claims yet

## P0 — DOM model

- [x] arena-based stable `NodeId`
- [x] parent/children relations
- [ ] mutation API with invariant checks
- [ ] element namespace representation
- [ ] interned atom/string strategy ADR
- [ ] explicit document lifecycle and generation IDs

### Exit condition

DOM ownership must not assume that renderer, networking or host live in the same process.

## P0 — parsing boundary

- [x] bootstrap parser behind `rarog_html::parse`
- [ ] define streaming input abstraction
- [ ] define parser error/reporting model
- [ ] ADR: standards parser strategy
- [ ] replace bootstrap parser with standards-oriented implementation/adapter

## P0 — style system boundary

- [x] `ComputedStyle` exists independently of layout
- [ ] stylesheet/source model
- [ ] selector representation
- [ ] cascade origin/layer/specificity data structures
- [ ] property ID/value representation
- [ ] style sharing/cache design note
- [ ] invalidation key prototype

## P0 — layout

- [x] block-like vertical bootstrap layout
- [ ] separate layout node identity from DOM node identity
- [ ] fragment tree type
- [ ] containing-block model
- [ ] margin/border/padding boxes
- [ ] intrinsic sizing interface
- [ ] text run abstraction (without committing to a shaping backend)
- [ ] dirty/invalidation flags

### Required invariant

Layout state is derived and disposable. DOM must never depend on layout object addresses.

## P0 — paint

- [x] backend-neutral `DisplayList`
- [x] software framebuffer rasterizer
- [ ] stable command IDs / paint chunks
- [ ] damage rectangles
- [ ] clip commands
- [ ] stacking-context representation
- [ ] transforms/opacity representation
- [ ] retained display-list experiment

## P1 — engine/embedder API

- [x] `render_html` orchestration bootstrap
- [ ] `Engine` object
- [ ] `View` object
- [ ] navigation/request interfaces without networking implementation
- [ ] callbacks/events without UI assumptions
- [ ] host policy interface
- [ ] resource budget data model

Proposed shape:

```rust
let engine = Engine::builder().build()?;
let view = engine.create_view(ViewOptions::default())?;
view.load_html(html, BaseUrl::about_blank())?;
let frame = view.render(viewport)?;
```

## P1 — observability from day one

Every render should eventually expose timings for:

```text
parse
style
layout
paint-list
raster
peak temporary memory
persistent document memory
```

R0 may use wall-clock timers; later milestones replace these with a structured tracing system.

## R0 exit test

Given a committed HTML fixture, two runs on the same architecture/toolchain must produce:

1. an equivalent DOM snapshot;
2. an equivalent computed-style snapshot;
3. an equivalent layout snapshot;
4. an identical display list;
5. an identical framebuffer hash.

Only after this pipeline is deterministic do we start R1 standards work.
