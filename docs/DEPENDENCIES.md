# Dependency strategy

Rarog does not equate independence with rewriting mature infrastructure.

## Engine-owned

These are intended to define Rarog and should remain under Rarog architectural control:

- DOM integration model and Web platform bindings
- style/cascade architecture
- layout and fragmentation
- invalidation/task graph
- display list and retained rendering model
- compositor scheduling
- resource budgets/lifecycle
- process model, IPC protocol and capability broker
- embedder API
- compatibility subsystem

## Candidates for mature external components

Components should be selected by technical evaluation and isolated behind adapters:

- JavaScript/Wasm: SpiderMonkey through the isolated `rarog-script-spidermonkey` adapter
- GPU abstraction: `wgpu` behind compositor/platform adapters
- Unicode normalization/segmentation: Unicode-focused Rust crates
- text shaping: HarfRust behind the Rarog-owned shaping contract
- HTML parsing: `html5ever` behind the Rarog-owned HTML adapter
- WebIDL parsing: `weedle2` behind the normalized WebIDL frontend
- URL parsing and public-suffix data: `url` + `psl` behind Rarog-owned URL/origin/site types
- image/audio/video codecs: mature audited libraries/system frameworks when those subsystems are introduced
- TLS: mature platform/Rust TLS implementation; never custom cryptography

## Selected adapters

### `cssparser` 0.37

R1 uses the published `cssparser` 0.37 release as the CSS Syntax tokenizer/parser backend with default features disabled.

The dependency is limited to the private `rarog-css` syntax adapter. Rarog continues to own selector representation and matching, specificity, cascade, invalidation dependencies, typed property/value conversion and computed style. No `cssparser` type is part of a public Rarog API.

The dependency is MPL-2.0 licensed and satisfies the workspace Rust 1.85 build gate. The adapter is covered by malformed-input regression tests and a dedicated CSS stylesheet fuzz target. Upgrading the backend must preserve these boundaries and pass the same compatibility and deterministic-render gates.

### `html5ever` 0.39

R1 uses `html5ever` for standards-oriented HTML tokenization and tree construction behind `rarog-html`. The tree sink, tendril and parser types remain private to the adapter; output is converted into the Rarog-owned DOM and diagnostics types. The HTML parser has a dedicated fuzz target.

### `weedle2` 5.0.0

R2 pins package `weedle2` 5.0.0 (imported as `weedle`) behind `rarog-webidl::StandardsWebIdlFrontend`. The dependency parses syntax; Rarog owns normalization, validation, binding metadata and the public WebIDL IR. The frontend has a dedicated fuzz target.

### `url` 2.5.7 and `psl` 2.1.226

R2 pins `url` 2.5.7 and `psl` 2.1.226 inside `rarog-url`. External URL and public-suffix representations do not cross the crate boundary; callers consume Rarog-owned URL, origin and site identity types. URL parsing/resolution has a dedicated fuzz target.

### `harfrust` 0.13.3

R1 uses `harfrust` in `rarog-text-opentype` behind the Rarog-owned shaping interfaces. Font fallback, source ranges, bidi state, shaping requests and fragment identity remain engine-owned; the adapter consumes registered OpenType bytes and returns Rarog-owned shaped glyph data.

### `mozjs` 0.21.6

R2 pins optional `mozjs` 0.21.6 in the isolated `rarog-script-spidermonkey` crate. Ordinary engine crates depend only on `rarog-script`. SpiderMonkey builds are exercised through dedicated Linux and Windows feature lanes; the adapter is the intentional narrow exception to the workspace-wide no-`unsafe` policy.

### Windows platform adapters

The Windows platform crate currently uses `font-kit` 0.14.3 for system-font selection and `clipboard-win` 5.4.1 for clipboard integration. Both dependencies remain target-specific implementation details behind `rarog-platform` contracts. Windows GPU selection and presentation enable the DX12 backend of the same pinned `wgpu` 26.0.1 release used by the compositor adapter.

### `wgpu` 26.0.1

R3 pins `wgpu` 26.0.1 in the private `rarog-compositor-wgpu` adapter. The selected release remains compatible with Rarog's Rust 1.85 MSRV, while newer major releases require a newer compiler.

The adapter depends only on Rarog-owned compositor, paint and geometry contracts. It receives an already-created `wgpu::Device` and `wgpu::Queue`, applies full or partial frame plans to the retained deterministic software staging framebuffer, and uploads tightly packed RGBA8 pixels into an adapter-owned GPU texture. Window handles, surface creation, adapter/device selection and presentation are intentionally outside this crate and remain platform integration responsibilities.

Default `wgpu` backend features are disabled here because this crate does not choose a graphics API or create a device. Platform crates may enable the backend features they require. No `wgpu` type is exposed by DOM, CSS, layout, paint, engine or the backend-neutral compositor contract.

## Selection rules

A third-party dependency must have:

1. a narrow reason for inclusion;
2. an adapter boundary where practical;
3. license compatibility;
4. security/maintenance review;
5. benchmark impact understood for core-path dependencies.

Do not expose a backend's types in public Rarog APIs unless that backend is deliberately part of the stable contract.
