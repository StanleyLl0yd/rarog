# Rarog Roadmap

## R0 — Ember (current bootstrap)

Goal: deterministic `HTML → CSS → layout → display list → pixels` path.

Exit criteria:

- workspace crate boundaries established;
- DOM arena works;
- tiny block layout works;
- display list is independent of layout implementation;
- headless shell renders a fixture to an image;
- snapshot tests can be added without a window system.

## R1 — Flame

- replace bootstrap HTML parsing with a standards-oriented tokenizer/tree-builder adapter;
- real CSS tokenizer/parser and cascade foundations;
- selector matching architecture;
- block/inline formatting contexts;
- image resource abstraction;
- text shaping abstraction;
- invalidation graph prototype.

## R2 — Flight

- WebIDL pipeline;
- replaceable script runtime abstraction;
- SpiderMonkey integration;
- events/event loop;
- DOM mutation invalidation;
- Fetch foundation;
- URL/origin/security primitives.

## R3 — Wings

- flexbox/grid milestones;
- retained display list;
- compositor thread;
- `wgpu` graphics backend;
- async image decode;
- scroll tree;
- frame scheduler.

## R4 — Sky

- Host process;
- Site process;
- IPC protocol;
- sandboxing;
- capability broker;
- site isolation by default;
- crash recovery.

## R5 — Web

- storage process;
- workers/service workers;
- WebSocket;
- audio/video;
- canvas/WebGL;
- accessibility foundation.

## R6 — Compat

- WPT dashboard;
- real-Web corpus;
- signed compatibility profiles;
- high-priority Web app scenarios;
- WebDriver + BiDi.

## R7 — View

- stable Rarog View C ABI;
- C++ wrapper;
- platform bindings;
- embedder lifecycle/permissions API;
- resource budgets exposed to embedders.

## R8 — Zorya Alpha

- Zorya becomes the reference desktop browser;
- user-facing browser work starts only after the engine is sufficiently useful.

## R9 — Rarog 1.0

1.0 is based on compatibility, security and resource targets, not elapsed time or feature count.
