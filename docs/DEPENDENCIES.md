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

## Selection rules

A third-party dependency must have:

1. a narrow reason for inclusion;
2. an adapter boundary where practical;
3. license compatibility;
4. security/maintenance review;
5. benchmark impact understood for core-path dependencies.

Do not expose a backend's types in public Rarog APIs unless that backend is deliberately part of the stable contract.
