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

- JavaScript/Wasm: SpiderMonkey initially
- GPU abstraction: `wgpu` candidate
- Unicode normalization/segmentation: Unicode-focused Rust crates
- text shaping: HarfBuzz or a mature Rust shaping stack behind an adapter
- image/audio/video codecs: mature audited libraries/system frameworks
- TLS: mature platform/Rust TLS implementation; never custom cryptography
- URL parsing: standards-oriented library if semantics fit Rarog's security model

## Selected adapters

### `cssparser` 0.37

R1 uses the published `cssparser` 0.37 release as the CSS Syntax tokenizer/parser backend with default features disabled.

The dependency is limited to the private `rarog-css` syntax adapter. Rarog continues to own selector representation and matching, specificity, cascade, invalidation dependencies, typed property/value conversion and computed style. No `cssparser` type is part of a public Rarog API.

The dependency is MPL-2.0 licensed and satisfies the workspace Rust 1.85 build gate. The adapter is covered by malformed-input regression tests and a dedicated CSS stylesheet fuzz target. Upgrading the backend must preserve these boundaries and pass the same compatibility and deterministic-render gates.

## Selection rules

A third-party dependency must have:

1. a narrow reason for inclusion;
2. an adapter boundary where practical;
3. license compatibility;
4. security/maintenance review;
5. benchmark impact understood for core-path dependencies.

Do not expose a backend's types in public Rarog APIs unless that backend is deliberately part of the stable contract.
