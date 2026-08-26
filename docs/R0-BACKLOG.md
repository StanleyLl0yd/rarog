# R0 — Ember backlog

R0 has one purpose: prove the shape of the engine with a deterministic end-to-end rendering path.

Windows is the primary platform lane for R0. Linux remains a portability lane so engine-core code does not accidentally depend on Windows APIs.

## P0 — repository health

- [x] Cargo workspace and crate boundaries
- [x] workspace-wide `unsafe_code = forbid`
- [x] CI: fmt/check/clippy/test/bootstrap render
- [x] Windows-primary CI lane
- [x] Linux portability CI lane
- [x] architectural decision records
- [x] unit tests for DOM arena invariants
- [x] parser fixture/invariant test
- [x] deterministic DOM/style/layout/fragment/display-list snapshot coverage
- [x] deterministic framebuffer hash and combined render-signature hash
- [x] explicit deterministic-render CI gate
- [ ] benchmark harness with no performance claims yet

## P0 — DOM model

- [x] arena-based stable `NodeId`
- [x] parent/children relations
- [x] checked mutation API with invariant checks
- [x] reparent/detach operations
- [x] element attribute and text mutation primitives
- [x] explicit document generation ID
- [x] generation-ordered mutation records for invalidation consumers
- [ ] element namespace representation
- [ ] interned atom/string strategy ADR

### Exit condition

DOM ownership must not assume that renderer, networking or host live in the same process.

## P0 — parsing boundary

- [x] bootstrap parser behind `rarog_html::parse`
- [x] parser output checked against DOM invariants in debug/test paths
- [ ] define streaming input abstraction
- [ ] define parser error/reporting model
- [ ] ADR: standards parser strategy
- [ ] replace bootstrap parser with standards-oriented implementation/adapter

## P0 — style system boundary

- [x] `ComputedStyle` exists independently of layout
- [x] bootstrap margin/padding/border edge values
- [x] stylesheet/source model
- [x] selector representation for bootstrap type/class/ID selectors
- [x] cascade origin/layer/specificity/source-order data structures
- [x] typed bootstrap property ID/value representation
- [x] user-agent + author `<style>` + inline style origins
- [x] selector invalidation key prototype
- [x] DOM-mutation-to-style/layout/paint invalidation primitives
- [x] persistent dirty state survives across DOM mutations until a render consumes it
- [ ] style sharing/cache design note
- [ ] descendant/sibling selector invalidation dependencies
- [ ] standards-oriented CSS tokenizer/parser adapter

## P0 — layout

- [x] block-like vertical bootstrap layout
- [x] separate `LayoutNodeId` from DOM `NodeId`
- [x] separate `FragmentId`
- [x] derived `LayoutTree`
- [x] derived `FragmentTree`
- [x] explicit content/padding/border/margin boxes
- [x] deterministic Layout Tree / computed-style / Fragment Tree snapshots
- [x] first paint-only incremental reuse experiment with persistent Layout/Fragment geometry
- [ ] containing-block model beyond the bootstrap available-width input
- [ ] intrinsic sizing interface
- [ ] text run abstraction (without committing to a shaping backend)
- [ ] incremental relayout application for geometry-affecting dirty nodes
- [ ] fragmentation cases that produce multiple fragments per layout node

### Required invariant

Layout state and fragment state are derived and disposable. DOM must never depend on layout object addresses or layout/fragment IDs.

## P0 — paint

- [x] backend-neutral `DisplayList`
- [x] software framebuffer rasterizer
- [x] paint consumes Fragment Tree rather than drawing from layout code
- [x] bootstrap background and border painting
- [x] stable deterministic display-item IDs for current fragment commands
- [x] damage rectangles by comparing previous/current display lists
- [x] deterministic display-list snapshot
- [x] stable framebuffer hash
- [ ] clip commands
- [ ] stacking-context representation
- [ ] transforms/opacity representation
- [ ] retained display-list experiment
- [ ] damage-scoped raster update instead of full framebuffer rerasterization

## P0 — platform boundary

- [x] Windows-first platform policy documented
- [x] engine core remains platform-neutral in R0
- [x] Windows-primary + Linux-portability CI policy
- [ ] Windows host/platform crate boundary
- [ ] window/event adapter
- [ ] font/text platform adapter
- [ ] input/IME adapter
- [ ] accessibility adapter
- [ ] sandbox/process adapter
- [ ] GPU/compositor platform adapter

### Required invariant

Win32, WinRT, Direct3D and other Windows-specific APIs must not leak into DOM, HTML, CSS, layout or script-facing Web platform crates.

## P1 — engine/embedder API

- [x] `render_html` orchestration bootstrap
- [x] previous-display-list input and damage output for bootstrap rendering
- [x] stateful R0 `RenderSession` bootstrap for mutation → dirty state → update orchestration
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
layout-tree
fragment
paint-list
raster
peak temporary memory
persistent document memory
```

Incremental frames must additionally expose which path ran (`unchanged`, `paint-only reuse`, `full rebuild`) and how many nodes were dirtied/patched.

R0 may use wall-clock timers; later milestones replace these with a structured tracing system.

## R0 exit test

Given a committed HTML fixture, repeated runs on the same architecture/toolchain must produce:

1. an equivalent DOM snapshot;
2. an equivalent stylesheet/computed-style snapshot;
3. an equivalent Layout Tree snapshot;
4. an equivalent Fragment Tree + box-model snapshot;
5. an identical display list with identical display-item IDs;
6. an identical framebuffer hash;
7. an identical combined deterministic render-signature hash.

The stateful R0 path must also prove that a paint-only computed-style mutation can reuse existing Layout/Fragment geometry, while a geometry-affecting or structural mutation falls back to a deterministic full rebuild.

The same deterministic test must pass in the Windows-primary CI lane. Only after the R0 pipeline and remaining bootstrap interfaces are stable do we start R1 standards work.
