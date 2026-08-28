# Rarog Web Engine

**A small engine for a big Web.**

Rarog is an experimental, independent, Rust-first Web engine designed around four priorities:

1. real-world compatibility;
2. low resource use;
3. strong isolation and capability-based security;
4. embeddability.

The **primary target platform is Windows**. Rarog is being designed so the engine core can remain portable, but the first production-quality host integration, GPU/compositor path, sandboxing, text/input integration, accessibility work and reference browser will target **Windows 10/11** first.

The workspace version is **0.1.0**. **R0 — Ember is complete**: it establishes the deterministic rendering, invalidation, paint, embedder and platform ownership boundaries that later milestones build on. **R1 — Flame** is the next development milestone and begins replacing bootstrap parsing/layout/text pieces with standards-oriented implementations behind those boundaries.

> The bootstrap HTML/CSS parsers remain intentionally incomplete and are **not** standards implementations. R0 proves the adapter contracts; R1 replaces the bootstrap algorithms behind them.

## R0 exit pipeline

```text
decoded HTML input
   ↓
rarog-html
   ↓
rarog-dom + mutation generations
   ↓
stylesheet sources → selectors → cascade/specificity
   ↓
computed style + invalidation dependencies
   ↓
persistent dirty state
   ├─ unchanged → no render work
   ├─ paint-only style change → retained Layout/Fragment geometry + paint patch
   ├─ footprint-safe geometry change → subtree Fragment relayout
   ├─ vertical-footprint geometry change → retained Layout Tree + flow-aware suffix relayout
   └─ unsafe structural/text/display-membership change → deterministic fallback/rebuild
   ↓
layout tree
   ↓
fragment tree + box model
   ↓
structural display scopes + stable display-item IDs + damage tracking
   ↓
software framebuffer / deterministic hash
```

The completed R0 foundation includes:

- checked DOM mutations, mutation records, document generation tracking and mutation-history pruning;
- explicit element namespaces and an atom/string ownership boundary;
- decoded streaming HTML input plus deterministic parser diagnostics behind a replaceable bootstrap parser;
- stylesheet/source, selector, property/value and cascade-priority structures;
- simple type/class/ID bootstrap matching plus selector invalidation keys and relational dependency metadata;
- persistent engine-owned dirty state across mutations and renders;
- a stateful `RenderSession` with paint-only reuse, footprint-safe subtree Fragment relayout, flow-aware vertical suffix relayout, conservative geometry fallback and deterministic full rebuild;
- explicit containing-block and intrinsic-size boundaries;
- grapheme-safe text ranges, line fragmentation, bidi runs, font fallback, shaping segmentation and backend-neutral shaping request/glyph result contracts;
- separate DOM, layout-node and fragment identities with derived/disposable layout state;
- a backend-neutral display list with clip, stacking, transform and opacity scopes;
- deterministic display-item IDs, retained-range validation, damage comparison and damage-scoped software framebuffer updates;
- bounded framebuffer allocation and a fallible public render boundary;
- render-stage timings, structural counters and a benchmark harness with no performance thresholds or claims;
- `Engine`/`View`, request forwarding, host policy, UI-neutral callbacks and enforced source/viewport resource budgets;
- `rarog-platform` plus the Windows-specific `rarog-platform-windows` ownership seam;
- deterministic DOM/style/layout/fragment/display-list snapshots and framebuffer/signature hashes;
- Windows-primary CI, Linux portability CI, an explicit Rust 1.85 MSRV check and immutable action pins;
- a dedicated R0 exit gate that rejects new unchecked Ember checklist items.

R0 deliberately does **not** claim general Web compatibility, standards completeness, production security, performance leadership or browser readiness. Those are later milestones with their own measurable exit criteria.

## Platform strategy

**Windows is first, not Windows-only.**

The engine-owned Web platform code stays independent of Win32/WinRT/D3D-specific APIs. Windows-specific code lives behind narrow platform adapters so Linux and macOS ports remain possible later without forcing the core architecture to follow the lowest common denominator.

R0 makes that separation concrete: `rarog-platform` defines the platform-neutral host capability contract and `rarog-platform-windows` is the first target-specific boundary. Concrete Windows font/text, input/IME, GPU, sandbox/process and accessibility services are enabled only when their later roadmap milestones implement them.

The first reference browser, **Zorya Browser**, is also planned for Windows first.

## Workspace

- `rarog-types` — shared geometry/color/value types
- `rarog-dom` — DOM arena, checked mutations, mutation records and generation tracking
- `rarog-html` — HTML input/diagnostics boundary plus bootstrap parser
- `rarog-css` — bootstrap stylesheet sources, selectors, cascade, computed style and invalidation primitives
- `rarog-layout` — derived Layout Tree, Fragment Tree, box model and text foundations
- `rarog-paint` — structural display list, stable IDs, damage tracking and software rasterizer
- `rarog-platform` — platform-neutral host capability contract
- `rarog-platform-windows` — Windows-specific host boundary for later concrete adapters
- `rarog-engine` — rendering, persistent incremental session, observability and embedder boundary
- `rarog-shell` — minimal CLI test shell

See `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/R0-BACKLOG.md`, `docs/R0-EXIT.md` and `docs/adr/`.

## Development checks

The same core checks used by CI include:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rarog-engine --test r0_exit
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

**R0 — Ember is complete. R1 — Flame is next.** Rarog remains experimental; no compatibility, performance, security-hardening or production-readiness claims are made yet.

Created by **Stanley Lloyd**. Contributions are welcome; see `CONTRIBUTING.md`.
