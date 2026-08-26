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
   ├─ paint-only style change → reuse Layout/Fragment geometry
   └─ structural/geometry change → deterministic full rebuild
   ↓
layout tree
   ↓
fragment tree + box model
   ↓
stable display-item IDs + damage tracking
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
- a stateful `RenderSession` that performs a real paint-only incremental reuse path when geometry is unchanged and falls back conservatively to a full derived-tree rebuild otherwise;
- separate DOM, layout-node and fragment identities;
- a derived/disposable Fragment Tree and explicit content/padding/border/margin boxes;
- stable display-item IDs and damage rectangles between display lists;
- deterministic DOM/style/layout/fragment/display-list snapshots and framebuffer/signature hashes;
- CI with Windows as the primary platform lane and Linux as a portability lane.

The first incremental experiment is intentionally narrow. It proves that dirty state can survive across frames and that existing Layout Tree / Fragment Tree geometry can be reused safely for paint-only computed-style changes. It does **not** yet claim incremental relayout, retained display-list updates, standards-complete invalidation or performance gains.

## Platform strategy

**Windows is first, not Windows-only.**

The engine-owned Web platform code must stay independent of Win32/WinRT/D3D-specific APIs. Windows-specific code will live behind narrow platform adapters so Linux and macOS ports remain possible later without forcing the core architecture to follow the lowest common denominator.

The first reference browser, **Zorya Browser**, is also planned for Windows first.

## Workspace

- `rarog-types` — shared geometry/color/value types
- `rarog-dom` — DOM arena, checked mutations, mutation records and generation tracking
- `rarog-html` — bootstrap HTML adapter
- `rarog-css` — bootstrap stylesheet sources, selectors, cascade, computed style and invalidation primitives
- `rarog-layout` — derived Layout Tree, Fragment Tree, box model and deterministic snapshots
- `rarog-paint` — stable display-item IDs, damage tracking, display list and software rasterizer
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
cargo run -p rarog-shell -- examples/hello.html rarog.ppm
```

## License

Rarog is dual-licensed under **Apache-2.0 OR MIT**, at your option. See `LICENSE-APACHE` and `LICENSE-MIT`.

## Project status

Rarog is in **R0 — Ember**, an early bootstrap milestone. No compatibility, performance, or production-readiness claims are made yet.

Created by **Stanley Lloyd**. Contributions are welcome; see `CONTRIBUTING.md`.
