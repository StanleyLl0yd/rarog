# Rarog Roadmap

## Platform priority

The first production target for both **Rarog Web Engine** and **Zorya Browser** is **Windows 10/11**.

The engine core remains portable by design. Linux is kept as an early portability/CI target, while macOS becomes an active target later. Platform-specific APIs must stay behind adapters so Windows-first delivery does not turn into Windows-coupled Web semantics.

## R0 — Ember (current bootstrap)

Goal: deterministic `HTML → DOM → style → Layout Tree → Fragment Tree → display list → pixels` path.

Current foundation:

- workspace crate boundaries established;
- checked DOM mutation API and invariant validation;
- document generation tracking;
- separate DOM, layout-node and fragment identities;
- explicit content/padding/border/margin boxes;
- display list independent of direct layout drawing;
- headless shell renders a fixture to an image;
- Windows-primary CI plus Linux portability CI.

Remaining R0 exit work:

- stylesheet/source model and selector/cascade foundations;
- invalidation primitives;
- damage tracking primitives;
- deterministic DOM/style/layout/fragment/display-list snapshots;
- deterministic framebuffer hash;
- benchmark harness with no performance claims;
- clearer containing-block/intrinsic-sizing interfaces.

## R1 — Flame

- replace bootstrap HTML parsing with a standards-oriented tokenizer/tree-builder adapter;
- real CSS tokenizer/parser and cascade foundations;
- selector matching architecture;
- block/inline formatting contexts;
- image resource abstraction;
- text shaping abstraction;
- invalidation graph prototype;
- first Windows text/font platform adapter.

## R2 — Flight

- WebIDL pipeline;
- replaceable script runtime abstraction;
- SpiderMonkey integration;
- events/event loop;
- DOM mutation invalidation;
- Fetch foundation;
- URL/origin/security primitives;
- Windows input/IME and clipboard host adapters.

## R3 — Wings

- flexbox/grid milestones;
- retained display list;
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
- user-facing browser work starts only after the engine is sufficiently useful.

## R9 — Rarog 1.0

1.0 is based on compatibility, security and resource targets, not elapsed time or feature count.

Windows is the first release-quality platform for 1.0. Other platforms may reach release quality before or after 1.0 depending on contributor capacity and measured engine maturity.
