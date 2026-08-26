# Rarog Web Engine

**A small engine for a big Web.**

Rarog is an experimental, independent, Rust-first Web engine designed around four priorities:

1. real-world compatibility;
2. low resource use;
3. strong isolation and capability-based security;
4. embeddability.

This repository is the **v0.1 bootstrap workspace**. It deliberately implements only a tiny deterministic rendering path so that the public interfaces between DOM, CSS, layout, paint and the host are established before JavaScript, GPU rendering and multi-process isolation are introduced.

> The bootstrap parser and stylesheet parser are intentionally incomplete and are **not** intended to become the standards implementation. They are scaffolding for the first end-to-end pipeline and will be replaced behind stable interfaces.

## v0.1 pipeline

```text
HTML source
   ↓
rarog-html
   ↓
rarog-dom
   ↓
rarog-css
   ↓
rarog-layout
   ↓
rarog-paint
   ↓
software framebuffer / PPM
```

## Workspace

- `rarog-types` — shared geometry/color/value types
- `rarog-dom` — DOM arena and node model
- `rarog-html` — bootstrap HTML adapter
- `rarog-css` — bootstrap CSS parser + style resolution
- `rarog-layout` — block layout prototype
- `rarog-paint` — display list + software rasterizer
- `rarog-engine` — public orchestration API
- `rarog-shell` — minimal CLI test shell

See `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` and `docs/adr/`.

## License

Rarog is dual-licensed under **Apache-2.0 OR MIT**, at your option. See `LICENSE-APACHE` and `LICENSE-MIT`.

## Project status

Rarog is in **R0 — Ember**, an early bootstrap milestone. No compatibility, performance, or production-readiness claims are made yet.

Created by **Stanley Lloyd**. Contributions are welcome; see `CONTRIBUTING.md`.
