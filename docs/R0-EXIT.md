# R0 — Ember exit audit

Status: **complete** once this document's merge commit passes the normal post-merge `main` CI.

R0 exists to prove the architecture and deterministic ownership boundaries of Rarog, not standards breadth or production readiness. The exit decision is therefore based on reproducible engine invariants, explicit fallback behavior, host/embedder seams and platform neutrality.

## What R0 proves

### Deterministic end-to-end rendering

The committed bootstrap path is:

```text
decoded HTML input
  → DOM
  → stylesheet/cascade
  → derived Layout Tree
  → derived Fragment Tree
  → display list + damage
  → software framebuffer
```

Repeated renders on the same architecture/toolchain are covered by deterministic DOM/style/layout/fragment/display-list snapshots, framebuffer hashing and a combined render-signature hash. Timing observations are deliberately excluded from deterministic identity.

### Stateful invalidation and reuse

`RenderSession` demonstrates that retained state can be updated without making DOM depend on renderer identities. R0 has explicit paths for unchanged frames, paint-only retained updates, footprint-safe subtree relayout, root-flow suffix relayout, conservative geometry fallback and deterministic full rebuild.

### Ownership boundaries

R0 establishes separate DOM, layout-node, fragment and display-item identities. DOM mutation history has an engine-owned consumption checkpoint. Parser input/diagnostics, style invalidation dependencies, text shaping requests, paint structural scopes and host resource budgets all have replaceable boundaries rather than leaking bootstrap implementation details across crates.

### Embedder boundary

`Engine` and `View` sit above `RenderSession`. Navigation/resource requests can be forwarded or blocked without implementing a network stack, callbacks do not assume a UI toolkit, and source/viewport resource budgets are enforced at the host-facing boundary.

### Platform boundary

`rarog-platform` is platform-neutral. `rarog-platform-windows` is the first target-specific host seam and is exercised by Windows CI without introducing Win32/WinRT/Direct3D dependencies into DOM, HTML, CSS, layout or engine-core semantics.

### Observability without claims

R0 exposes wall-clock stage timings, structural counters, incremental-path reports and a reproducible benchmark harness. There are no latency thresholds or public performance claims. Peak/persistent allocator-backed byte accounting is deferred until a trustworthy measurement boundary exists.

## Explicitly not required for R0 exit

The following work is intentionally deferred rather than treated as incomplete Ember work:

- WHATWG-oriented HTML tokenizer/tree builder — R1
- standards-oriented CSS tokenizer/parser and broader selectors/cascade — R1
- production OpenType shaping and Windows font/text adapter — R1
- script runtime/WebIDL/events/input/IME — R2
- GPU compositor and Windows GPU integration — R3
- multi-process site isolation and Windows sandbox/process hardening — R4
- accessibility implementation and Windows bridge — R5
- reference browser window/UI integration — R8

This separation matters: R0 is complete only because its goal is architectural proof. It must not be described as standards-complete, compatible with the general Web, secure for hostile content, performance-competitive or production-ready.

## Automated exit gate

`crates/rarog-engine/tests/r0_exit.rs` provides an explicit milestone gate. It verifies that the R0 backlog has no unchecked checklist entries and re-checks the deterministic end-to-end render contract. Both Windows-primary and Linux-portability CI run it. The existing workspace, determinism, incremental, correctness-hardening, bootstrap-render and MSRV gates remain authoritative as well.

The R0 backlog is historical scope documentation after exit. New work should be added to `docs/ROADMAP.md` under the appropriate later milestone instead of reopening R0 unless an actual Ember invariant is found to be incorrect.

## Release identity

The workspace remains version `0.1.0`. After the exit PR and its post-merge `main` CI are green, that merge commit is the canonical source point for the `r0-ember` milestone tag.
