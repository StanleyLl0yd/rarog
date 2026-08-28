# R0 — Ember backlog

Status: **complete**.

R0 has one purpose: prove the shape of the engine with a deterministic end-to-end rendering path and stable ownership boundaries before standards breadth, JavaScript, GPU composition and multi-process isolation expand the implementation.

Windows is the primary platform lane for R0. Linux remains a portability lane so engine-core code does not accidentally depend on Windows APIs.

The R0 exit contract is checked by `cargo test -p rarog-engine --test r0_exit` and documented in `docs/R0-EXIT.md`.

## P0 — repository health

- [x] Cargo workspace and crate boundaries
- [x] workspace-wide `unsafe_code = forbid`
- [x] CI: fmt/check/clippy/test/bootstrap render
- [x] Windows-primary CI lane
- [x] Linux portability CI lane
- [x] architectural decision records
- [x] unit tests for DOM arena invariants
- [x] deterministic mutation-stress test over checked DOM operations
- [x] parser fixture/invariant test
- [x] deterministic DOM/style/layout/fragment/display-list snapshot coverage
- [x] deterministic framebuffer hash and combined render-signature hash
- [x] explicit deterministic-render CI gate
- [x] explicit incremental reuse/fallback CI gate
- [x] explicit Rust 1.85 MSRV check
- [x] immutable CI action pinning
- [x] benchmark harness with no performance claims yet
- [x] dedicated R0 exit-manifest and render-contract CI gate

## P0 — DOM model

- [x] arena-based stable `NodeId`
- [x] parent/children relations
- [x] checked mutation API with invariant checks
- [x] reparent/detach operations
- [x] element attribute and text mutation primitives
- [x] explicit document generation ID
- [x] generation-ordered mutation records for invalidation consumers
- [x] mutation-history pruning/checkpoint after the active engine consumer advances
- [x] engine-owned mutation facade prevents session callers from pruning mutation history
- [x] element namespace representation
- [x] atom/string ownership strategy ADR

### Exit condition

DOM ownership does not assume that renderer, networking or host live in the same process. Layout/paint identities remain derived and disposable.

## P0 — parsing boundary

- [x] bootstrap parser behind `rarog_html::parse`
- [x] parser output checked against DOM invariants in debug/test paths
- [x] decoded streaming-input abstraction
- [x] deterministic parser diagnostic/error model
- [x] standards-parser strategy ADR

The standards-oriented HTML tokenizer/tree-builder is intentionally an **R1 — Flame** implementation task, not an R0 exit blocker.

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
- [x] non-finite bootstrap CSS lengths are rejected before computed geometry
- [x] style-sharing/cache ownership design
- [x] descendant/following-sibling invalidation dependency model

The standards-oriented CSS tokenizer/parser and broader selector grammar are **R1 — Flame** work.

## P0 — layout and text foundations

- [x] block-like vertical bootstrap layout
- [x] separate `LayoutNodeId` from DOM `NodeId`
- [x] separate `FragmentId`
- [x] derived `LayoutTree`
- [x] derived `FragmentTree`
- [x] explicit content/padding/border/margin boxes
- [x] deterministic Layout Tree / computed-style / Fragment Tree snapshots
- [x] first paint-only incremental reuse experiment with persistent Layout/Fragment geometry
- [x] containing-block model foundation beyond a raw available-width argument
- [x] intrinsic sizing interface
- [x] text-run abstraction and scalar-indexed source ranges
- [x] grapheme-safe text boundaries and line fragmentation
- [x] bidi-run foundation and visual ordering boundary
- [x] font fallback runs
- [x] shaping segmentation across bidi/font/script boundaries
- [x] backend-neutral shaping request/glyph result boundary
- [x] first geometry-affecting incremental relayout from a retained Layout Tree
- [x] first subtree-local incremental relayout for geometry changes that preserve vertical flow footprint
- [x] first ancestor/sibling-aware local reflow for vertical-footprint changes in the root block-flow context
- [x] bootstrap text fragmentation can produce multiple fragments per layout node with stable ordinals

### Required invariant

Layout state and fragment state are derived and disposable. DOM never depends on layout object addresses or layout/fragment IDs.

A production OpenType shaper, platform font discovery and standards-complete Unicode algorithms remain later milestones.

## P0 — paint

- [x] backend-neutral `DisplayList`
- [x] software framebuffer rasterizer
- [x] paint consumes Fragment Tree rather than drawing from layout code
- [x] bootstrap background and border painting
- [x] deterministic display-item IDs combine source identity, fragment identity and paint slot
- [x] display-list ID uniqueness invariant prevents silent damage-index collisions
- [x] damage rectangles by comparing previous/current display lists
- [x] checked framebuffer allocation with an explicit R0 pixel budget
- [x] deterministic display-list snapshot
- [x] stable framebuffer hash
- [x] clip commands with nested software-raster clip-stack semantics and conservative damage fallback
- [x] stacking-context representation with explicit balanced display-list scopes
- [x] transform and opacity representation
- [x] retained display-list replacement experiment for affected fragment subtrees
- [x] retained display-list v2 uses exact contiguous ranges and preserves structural scope balance
- [x] damage-scoped software raster update instead of full framebuffer rerasterization
- [x] fragmentation/stacking/clip/transform-aware retained-range validation

## P0 — platform boundary

- [x] Windows-first platform policy documented
- [x] engine core remains platform-neutral in R0
- [x] Windows-primary + Linux-portability CI policy
- [x] `rarog-platform` neutral host/capability boundary
- [x] `rarog-platform-windows` target-specific host boundary
- [x] Windows-specific APIs do not leak into DOM/HTML/CSS/layout/engine-core semantics

Concrete services are intentionally staged after R0:

- Windows font/text adapter — R1
- input/IME and event-loop integration — R2
- GPU/compositor adapter — R3
- sandbox/process adapter — R4
- accessibility bridge — R5
- reference-browser window/UI integration — R8

## P1 — engine/embedder API

- [x] `render_html` orchestration bootstrap
- [x] previous-display-list input and damage output for bootstrap rendering
- [x] stateful R0 `RenderSession` mutation → dirty state → update orchestration
- [x] fallible render/session construction for invalid or oversized framebuffer viewports
- [x] `Engine` object
- [x] `View` object
- [x] navigation/request interfaces without networking implementation
- [x] callbacks/events without UI assumptions
- [x] host policy interface
- [x] enforced resource-budget data model

## P1 — observability from day one

- [x] wall-clock stage timings for parse/style/layout-tree/fragment/paint-list/raster/total
- [x] structural render counters for DOM/layout/fragments/display commands/damage
- [x] incremental mode plus dirty/patched-node reporting
- [x] dependency-free R0 benchmark harness with no performance thresholds or claims

Allocator-backed peak/persistent byte accounting is deliberately deferred until there is a trustworthy measurement boundary.

## R0 exit test

The committed R0 fixture and CI prove:

1. equivalent DOM snapshots across repeated runs;
2. equivalent stylesheet/computed-style snapshots;
3. equivalent Layout Tree snapshots;
4. equivalent Fragment Tree + box-model snapshots;
5. identical display lists with identical display-item IDs;
6. identical framebuffer hashes;
7. identical combined deterministic render-signature hashes;
8. paint-only retained reuse;
9. footprint-safe subtree relayout;
10. ancestor/sibling-aware root-flow reflow for vertical-footprint changes;
11. conservative geometry/full-rebuild fallbacks when local reuse is not proven safe;
12. Windows-primary execution plus Linux portability and Rust 1.85 MSRV checks.

`crates/rarog-engine/tests/r0_exit.rs` additionally fails if this R0 backlog contains a new unchecked checklist item. New standards or platform breadth therefore belongs in the roadmap for the appropriate later milestone instead of silently reopening Ember.
