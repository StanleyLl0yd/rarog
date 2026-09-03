# Rarog Roadmap

## Platform priority

The first production target for both **Rarog Web Engine** and **Zorya Browser** is **Windows 10/11**.

The engine core remains portable by design. Linux is kept as an early portability/CI target, while macOS becomes an active target later. Platform-specific APIs stay behind adapters so Windows-first delivery does not turn into Windows-coupled Web semantics.

## R0 — Ember — complete

Goal: deterministic `HTML → DOM → style/cascade → Layout Tree → Fragment Tree → display list/damage → pixels` path plus the first stateful invalidation/reuse and host/embedder boundaries.

R0 exits with:

- checked DOM mutations, mutation records, generation tracking and mutation-history ownership;
- explicit element namespaces and atom/string ownership boundary;
- decoded streaming HTML input plus deterministic parser diagnostics behind a replaceable bootstrap parser;
- stylesheet/source, selector, cascade and typed bootstrap property/value structures;
- selector invalidation keys, relational dependency metadata and persistent engine-owned dirty state;
- separate DOM/layout/fragment identities and disposable derived layout state;
- containing-block, intrinsic-size, grapheme/bidi/font-fallback/shaping-request foundations;
- paint-only reuse, subtree relayout, root-flow suffix reflow and conservative deterministic fallbacks;
- backend-neutral display list, clip/stacking/transform/opacity scopes, retained-range validation and damage-scoped software raster;
- deterministic snapshots, framebuffer hash and combined render-signature hash;
- render observability plus a benchmark harness with no performance claims;
- `Engine`/`View`, request forwarding, host policy, callbacks and enforced resource budgets;
- platform-neutral `rarog-platform` plus the Windows-specific `rarog-platform-windows` host seam;
- Windows-primary CI, Linux portability CI and Rust 1.85 MSRV checks;
- a dedicated automated R0 exit gate.

See `R0-BACKLOG.md` and `R0-EXIT.md` for the completed scope and explicit deferrals.

## R1 — Flame — complete

- replace bootstrap HTML parsing with a standards-oriented tokenizer/tree-builder adapter;
- replace bootstrap CSS parsing with a standards-oriented tokenizer/parser adapter;
- expand selector matching to combinators, attributes and pseudo-classes in measured slices;
- real cascade details including importance, inheritance and CSS-wide values;
- block and inline formatting contexts;
- image resource abstraction;
- connect a production OpenType shaping backend to the existing shaping request/glyph boundary;
- first Windows font discovery/text platform adapter;
- expand the R0 invalidation graph into standards-aware incremental style/layout work;
- expand retained/damage-aware paint across richer formatting and stacking behavior.

See `R1-BACKLOG.md` and `R1-EXIT.md` for the completed scope, retained-rendering boundaries and explicit deferrals.

## R2 — Flight — in progress

- WebIDL pipeline;
- replaceable script runtime abstraction;
- SpiderMonkey integration;
- events/event loop;
- script-driven DOM mutation invalidation;
- Fetch foundation;
- URL/origin/security primitives;
- Windows input/IME and clipboard host adapters.

## R3 — Wings

- flexbox/grid milestones;
- compositor thread;
- `wgpu` graphics backend;
- Windows-first GPU/compositor integration;
- async image decode;
- scroll tree;
- frame scheduler.

## R4 — Sky

- Host process;
- Site process;
- IPC protocol;
- Windows sandbox/process hardening first;
- capability broker;
- site isolation by default;
- crash recovery.

## R5 — Web

- storage process;
- workers/service workers;
- WebSocket;
- audio/video;
- canvas/WebGL;
- accessibility foundation;
- Windows accessibility bridge first.

## R6 — Compat

- WPT dashboard;
- real-Web corpus;
- signed compatibility profiles;
- high-priority Web app scenarios;
- WebDriver + BiDi;
- Windows 10/11 real-machine compatibility runs.

## R7 — View

- stable Rarog View C ABI;
- C++ wrapper;
- Windows embedding binding first;
- additional platform bindings later;
- embedder lifecycle/permissions API;
- resource budgets exposed to embedders.

## R8 — Zorya Alpha

- Zorya becomes the reference desktop browser;
- first public target is Windows;
- concrete native window/UI integration is completed for the reference browser;
- user-facing browser work starts only after the engine is sufficiently useful.

## R9 — Rarog 1.0

1.0 is based on compatibility, security and resource targets, not elapsed time or feature count.

Windows is the first release-quality platform for 1.0. Other platforms may reach release quality before or after 1.0 depending on contributor capacity and measured engine maturity.
