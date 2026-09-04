# Rarog Web Engine

**A small engine for a big Web.**

Rarog is an experimental, independent, Rust-first Web engine designed around four priorities:

1. real-world compatibility;
2. low resource use;
3. strong isolation and capability-based security;
4. embeddability.

The **primary target platform is Windows**. Rarog is being designed so the engine core can remain portable, but the first production-quality host integration, GPU/compositor path, sandboxing, input integration, accessibility work and reference browser will target **Windows 10/11** first.

The workspace version is **0.1.0**. **R0 — Ember is complete**: it established deterministic rendering, invalidation, paint, embedder and platform ownership boundaries. **R1 — Flame is complete**: it replaced the bootstrap HTML/CSS paths with standards-oriented adapters, established scoped block/inline formatting foundations, connected production OpenType shaping and Windows font discovery, and broadened retained incremental rendering and damage-aware paint. **R2 — Flight is complete**: it established normalized WebIDL and replaceable script-runtime boundaries, the first SpiderMonkey adapter, events and engine-owned task/microtask scheduling, URL/origin/Fetch foundations, script-driven retained rendering checkpoints, and Windows input/IME/clipboard adapters. **R3 — Wings is in progress**, beginning with measured flexbox/layout work before compositor/GPU, asynchronous resources and scrolling.

> Rarog has standards-oriented foundations, not a claim of general-Web compatibility or standards completeness. Broad DOM/Web API bindings, mature script/network integration, flex/grid completeness, GPU/compositor, process isolation and browser readiness remain roadmap work.

## Current engine pipeline

```text
decoded HTML input
   ↓
standards-oriented rarog-html adapter
   ↓
rarog-dom + mutation generations
   ↓
standards-oriented CSS parsing → selectors → cascade/inheritance
   ↓
computed style + invalidation dependencies
   ↓
persistent dirty state
   ├─ unchanged → no render work
   ├─ paint-only style change → retained Layout/Fragment geometry + paint patch
   ├─ local geometry change → subtree Fragment relayout when proven safe
   ├─ text/structure/formatting-context change → retained Layout refresh + flow-aware relayout
   └─ unprovable retained state → deterministic fail-closed fallback/rebuild
   ↓
layout tree
   ↓
fragment tree + block/inline formatting foundations
   ↓
retained display list + structural scopes + stable display-item IDs
   ↓
damage-aware software framebuffer / deterministic hash
```

The completed R0–R2 foundation includes:

- checked DOM mutations, mutation records, document generation tracking and mutation-history pruning;
- explicit element namespaces and an atom/string ownership boundary;
- standards-oriented HTML tokenization/tree building through an `html5ever` adapter behind Rarog-owned DOM/parser types;
- standards-oriented CSS parsing plus combinators, attribute selectors, pseudo-classes, importance, inheritance and CSS-wide values;
- persistent engine-owned dirty state across mutations and renders;
- a stateful `RenderSession` with paint-only reuse, subtree Fragment relayout, retained parent/subtree refresh, flow-aware relayout and deterministic fail-closed fallback;
- scoped block formatting foundations including margin collapse, auto/min/max sizing and explicit BFC boundaries;
- scoped inline formatting foundations including shared line construction, baseline/vertical-align behavior and nested/multi-leaf inline fragmentation;
- explicit containing-block and intrinsic-size boundaries;
- production OpenType shaping behind a Rarog-owned shaping contract, plus Windows system-font discovery and a tested DirectWrite-selected face → HarfRust handoff;
- a bounded decoded-image resource abstraction with revision-aware paint identity;
- separate DOM, layout-node and fragment identities with derived/disposable layout state;
- a backend-neutral display list with clip, stacking, transform and opacity scopes;
- deterministic display-item IDs, retained-range/suffix validation, structural damage comparison and damage-scoped software framebuffer updates;
- bounded framebuffer allocation and a fallible public render boundary;
- render-stage timings, structural counters and benchmark harnesses with no performance thresholds or claims;
- `Engine`/`View`, request forwarding, host policy, UI-neutral callbacks and enforced source/viewport resource budgets;
- a normalized WebIDL IR and standards-oriented parser adapter behind Rarog-owned metadata;
- a replaceable script-runtime contract plus an isolated SpiderMonkey ESR backend with opaque realm/root identities;
- Event/EventTarget foundations and bounded engine-owned task/microtask scheduling connected to retained render checkpoints;
- Rarog-owned URL/origin/site identity and Fetch request/response/network-capability boundaries;
- platform-neutral keyboard, pointer, wheel, text-input and clipboard contracts with Windows input/IME/clipboard adapters;
- `rarog-platform` plus the Windows-specific `rarog-platform-windows` ownership seam;
- deterministic DOM/style/layout/fragment/display-list snapshots and framebuffer/signature hashes;
- Windows-primary CI, Linux portability CI, an explicit Rust 1.85 MSRV check, dedicated SpiderMonkey jobs and immutable action pins;
- dedicated automated R0, R1 and R2 exit gates.

Rarog deliberately does **not** claim general Web compatibility, standards completeness, production security, performance leadership or browser readiness. Those are later milestones with their own measurable exit criteria.

## Platform strategy

**Windows is first, not Windows-only.**

The engine-owned Web platform code stays independent of Win32/WinRT/D3D-specific APIs. Windows-specific code lives behind narrow platform adapters so Linux and macOS ports remain possible later without forcing the core architecture to follow the lowest common denominator.

R1 made the first production text platform path concrete, and R2 extended the Windows host seam through normalized input, IME and clipboard adapters. GPU/compositor, sandbox/process and accessibility services remain assigned to later roadmap milestones.

The first reference browser, **Zorya Browser**, is also planned for Windows first.

## Workspace

- `rarog-types` — shared geometry/color/value types
- `rarog-resources` — bounded decoded-resource ownership and revision identity
- `rarog-dom` — DOM arena, checked mutations, mutation records and generation tracking
- `rarog-events` — Event/EventTarget registration and dispatch foundations
- `rarog-html` — standards-oriented HTML adapter plus Rarog-owned input/diagnostics boundary
- `rarog-css` — standards-oriented CSS parsing, selectors, cascade, computed style and invalidation primitives
- `rarog-layout` — derived Layout Tree, Fragment Tree, block/inline/flex foundations and text layout
- `rarog-text-opentype` — production OpenType shaping adapter behind Rarog-owned contracts
- `rarog-webidl` — Rarog-owned normalized WebIDL IR, validation and parser frontend boundary
- `rarog-url` — Rarog-owned URL, origin and site identity primitives
- `rarog-fetch` — bounded Fetch values and embedder network-capability boundary
- `rarog-script` — replaceable script-runtime, realm and rooted-value contracts
- `rarog-script-spidermonkey` — isolated SpiderMonkey adapter behind `rarog-script`
- `rarog-scheduler` — bounded task and microtask scheduling primitives
- `rarog-paint` — retained structural display list, stable IDs, damage tracking and software rasterizer
- `rarog-platform` — platform-neutral host, font, input and clipboard capability contracts
- `rarog-platform-windows` — Windows-specific font, input, IME and clipboard adapters
- `rarog-engine` — rendering, persistent incremental session, event-loop bridge, observability and embedder boundary
- `rarog-shell` — minimal CLI test shell

See `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, the milestone backlog/exit documents under `docs/`, and `docs/adr/`.

## Development checks

The same core checks used by CI include:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rarog-engine --test r0_exit
cargo test -p rarog-engine --test p1_exit
cargo test -p rarog-engine --test r01_correctness
cargo test -p rarog-engine --test r1_exit
cargo test -p rarog-engine --test r2_exit
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo run -p rarog-shell -- examples/hello.html rarog.ppm
```

## License

Rarog is dual-licensed under **Apache-2.0 OR MIT**, at your option. See `LICENSE-APACHE` and `LICENSE-MIT`.

## Project status

**R0 — Ember, R1 — Flame and R2 — Flight are complete. R3 — Wings is in progress.** Rarog remains experimental; no compatibility, performance, security-hardening or production-readiness claims are made yet.

Created by **Stanley Lloyd**. Contributions are welcome; see `CONTRIBUTING.md`.
