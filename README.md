# Rarog Web Engine

**A small engine for a big Web.**

Rarog is an experimental, independent, Rust-first Web engine designed around four priorities:

1. real-world compatibility;
2. low resource use;
3. strong isolation and capability-based security;
4. embeddability.

The **primary target platform is Windows**. Rarog is being designed so the engine core can remain portable, but the first production-quality host integration, GPU/compositor path, sandboxing, text/input integration, accessibility work and reference browser will target **Windows 10/11** first.

This repository is the **v0.1 / R0 — Ember workspace**. It deliberately implements a small deterministic rendering path so that the public interfaces between DOM, style, layout, fragments, paint and the host are established before JavaScript, GPU rendering and multi-process isolation are introduced.

> The bootstrap HTML and inline-style implementations are intentionally incomplete and are **not** intended to become the standards implementation. They are scaffolding for the first end-to-end pipeline and will be replaced behind stable interfaces.

## Current R0 pipeline

```text
HTML source
   ↓
rarog-html
   ↓
rarog-dom
   ↓
rarog-css
   ↓
layout tree
   ↓
fragment tree + box model
   ↓
rarog-paint
   ↓
software framebuffer / PPM
```

The current R0 foundation now includes:

- checked DOM mutations and a document generation counter;
- a layout identity that is separate from DOM node identity;
- a derived/disposable Fragment Tree;
- explicit content, padding, border and margin boxes;
- bootstrap border/background painting through the display list;
- CI with Windows as the primary platform lane and Linux as a portability lane.

## Platform strategy

**Windows is first, not Windows-only.**

The engine-owned Web platform code must stay independent of Win32/WinRT/D3D-specific APIs. Windows-specific code will live behind narrow platform adapters so Linux and macOS ports remain possible later without forcing the core architecture to follow the lowest common denominator.

The first reference browser, **Zorya Browser**, is also planned for Windows first.

## Workspace

- `rarog-types` — shared geometry/color/value types
- `rarog-dom` — DOM arena, checked mutation API and generation tracking
- `rarog-html` — bootstrap HTML adapter
- `rarog-css` — bootstrap style resolution and box-edge values
- `rarog-layout` — derived Layout Tree, Fragment Tree and box model
- `rarog-paint` — display list + software rasterizer
- `rarog-engine` — public orchestration API
- `rarog-shell` — minimal CLI test shell

See `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/R0-BACKLOG.md` and `docs/adr/`.

## Development checks

The same checks used by CI are:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p rarog-shell -- examples/hello.html rarog.ppm
```

## License

Rarog is dual-licensed under **Apache-2.0 OR MIT**, at your option. See `LICENSE-APACHE` and `LICENSE-MIT`.

## Project status

Rarog is in **R0 — Ember**, an early bootstrap milestone. No compatibility, performance, or production-readiness claims are made yet.

Created by **Stanley Lloyd**. Contributions are welcome; see `CONTRIBUTING.md`.
