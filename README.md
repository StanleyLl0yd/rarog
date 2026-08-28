# Rarog Web Engine

**A small engine for a big Web.**

Rarog is an experimental, independent, Rust-first Web engine designed around four priorities:

1. real-world compatibility;
2. low resource use;
3. strong isolation and capability-based security;
4. embeddability.

The **primary target platform is Windows**. Rarog is being designed so the engine core can remain portable, but the first production-quality host integration, GPU/compositor path, sandboxing, text/input integration, accessibility work and reference browser will target **Windows 10/11** first.

This repository is the **v0.1 / R0 — Ember workspace**. It deliberately implements a small deterministic rendering path so that the public interfaces between DOM, style, layout, fragments, paint and the host are established before JavaScript, GPU rendering and multi-process isolation are introduced.

> The bootstrap HTML/CSS parsers are intentionally incomplete and are **not** intended to become the standards implementation. They are scaffolding for the first end-to-end pipeline and will be replaced behind stable interfaces.

## Current R0 pipeline

```text
HTML source
   ↓
rarog-html
   ↓
rarog-dom + mutation generations
   ↓
stylesheet sources → selectors → cascade/specificity
   ↓
computed style + invalidation keys
   ↓
persistent dirty state
   ├─ paint-only style change → reuse Layout/Fragment geometry + retained paint update
   ├─ footprint-safe geometry change → subtree Fragment relayout
   ├─ vertical-footprint geometry change → retain Layout Tree + flow-aware suffix relayout
   └─ structural/text/display-membership change → deterministic full rebuild
   ↓
layout tree
   ↓
fragment tree + box model
   ↓
source + fragment + paint-slot display-item IDs + damage tracking
   ↓
software framebuffer / deterministic hash
```

The current R0 foundation includes:

- checked DOM mutations, mutation records and document generation tracking;
- stylesheet/source, selector, property/value and cascade-priority data structures;
- simple type, class and ID selector matching for the bootstrap path;
- user-agent, author `<style>` and inline style origins with deterministic source order/specificity handling;
- selector invalidation keys plus DOM-mutation-to-style/layout/paint dirty primitives;
- persistent engine-owned dirty state across mutations and renders;
- a stateful `RenderSession` with paint-only reuse, footprint-safe subtree Fragment relayout, flow-aware vertical suffix relayout from a retained Layout Tree, whole-Fragment-Tree fallback when local flow mapping is not provably safe, and conservative full rebuild for structural/text/display-membership changes;
- explicit containing-block, intrinsic-size and text-run abstractions in the bootstrap layout path;
- separate DOM, layout-node and fragment identities;
- a derived/disposable Fragment Tree and explicit content/padding/border/margin boxes;
- display-item IDs combine stable source identity, fragment identity and paint slot; generated display lists enforce ID uniqueness before damage comparison, while retained replacement preserves unaffected ranges when safe;
- damage-scoped software framebuffer updates that clear and rerasterize only damaged rectangles;
- bounded framebuffer allocation with checked pixel counts and a fallible public render boundary for invalid or oversized viewports;
- mutation-journal pruning after the engine consumes a DOM generation, with `RenderSession` exposing a mutation-only `DocumentEditor` so callers cannot prune its journal behind the engine;
- deterministic DOM/style/layout/fragment/display-list snapshots and framebuffer/signature hashes;
- CI with Windows as the primary platform lane, Linux as a portability lane and an explicit Rust 1.85 MSRV check; CI actions are pinned to immutable revisions.

The incremental experiment is intentionally narrow. It now proves paint-only retained updates, subtree-local Fragment relayout for geometry changes that preserve vertical flow footprint, ancestor/sibling-aware suffix reflow for vertical-footprint changes in the current root block flow, conservative whole-Fragment-Tree fallback when that mapping is not safe, and damage-scoped software raster updates. It does **not** yet claim general CSS incremental reflow, nested formatting-context-local propagation, fragmentation-aware retained painting, standards-complete invalidation or measured performance gains.

## Platform strategy

**Windows is first, not Windows-only.**

The engine-owned Web platform code must stay independent of Win32/WinRT/D3D-specific APIs. Windows-specific code will live behind narrow platform adapters so Linux and macOS ports remain possible later without forcing the core architecture to follow the lowest common denominator.

R0 now makes that separation concrete: `rarog-platform` defines the platform-neutral host capability contract and `rarog-platform-windows` is the first target-specific boundary. The Windows crate does not yet advertise windowing, font, IME, accessibility, sandbox or GPU capabilities; those are enabled only when real adapters are implemented.

The first reference browser, **Zorya Browser**, is also planned for Windows first.

## Workspace

- `rarog-types` — shared geometry/color/value types
- `rarog-dom` — DOM arena, checked mutations, mutation records and generation tracking
- `rarog-html` — bootstrap HTML adapter
- `rarog-css` — bootstrap stylesheet sources, selectors, cascade, computed style and invalidation primitives
- `rarog-layout` — derived Layout Tree, Fragment Tree, box model and deterministic snapshots
- `rarog-paint` — stable display-item IDs, damage tracking, display list and software rasterizer
- `rarog-platform` — platform-neutral host capability contract
- `rarog-platform-windows` — Windows-specific host boundary for future adapters
- `rarog-engine` — stateless rendering plus persistent dirty-state / incremental-session orchestration
- `rarog-shell` — minimal CLI test shell

See `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/R0-BACKLOG.md` and `docs/adr/`.

## Development checks

The same core checks used by CI are:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rarog-engine deterministic_render_snapshot_and_hash
cargo test -p rarog-engine paint_only_update_reuses_layout_and_fragment_geometry
cargo test -p rarog-engine geometry_change_relayouts_without_rebuilding_layout_tree
cargo test -p rarog-engine vertical_geometry_change_reflows_ancestors_and_following_siblings
cargo test -p rarog-paint retained_display_patch_preserves_unrelated_items
cargo test -p rarog-paint damage_raster_matches_full_raster
cargo test -p rarog-css non_finite_lengths_are_rejected
cargo test -p rarog-dom deterministic_mutation_sequences_preserve_dom_invariants
cargo test -p rarog-engine invalid_viewport_is_reported_instead_of_panicking
cargo test -p rarog-paint fragment_component_prevents_multi_fragment_collisions
cargo run -p rarog-shell -- examples/hello.html rarog.ppm
```

## License

Rarog is dual-licensed under **Apache-2.0 OR MIT**, at your option. See `LICENSE-APACHE` and `LICENSE-MIT`.

## Project status

Rarog is in **R0 — Ember**, an early bootstrap milestone. No compatibility, performance, or production-readiness claims are made yet.

Created by **Stanley Lloyd**. Contributions are welcome; see `CONTRIBUTING.md`.
