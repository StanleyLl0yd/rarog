# ADR-0028: R0 observability and benchmark harness

## Status

Accepted.

## Context

R0 already has deterministic correctness gates and several incremental rendering paths, but it had no stable timing/counter boundary and no reproducible harness for exercising those paths. Ad-hoc wall-clock measurements are easy to misinterpret and must not become public performance claims.

## Decision

Full renders expose backend-neutral `RenderObservability` containing wall-clock stage timings and structural counters. Timing data is intentionally excluded from deterministic hashes and snapshots. The layout stage is split at a public `build_layout_tree` boundary so Layout Tree and Fragment Tree construction can be observed separately without changing their identities.

`IncrementalReport` carries total update elapsed time in addition to its existing mode, generation and dirty/patched-node counts. R0 does not attempt allocator instrumentation or fabricated memory-byte estimates; real peak and persistent memory accounting will require a later tracing/allocator boundary.

A dependency-free `rarog-engine` example provides fixed full-render, paint-only, subtree-relayout and flow-relayout scenarios. It accepts an iteration count and prints simple CSV-compatible samples. CI compiles the harness but does not enforce latency thresholds.

## Consequences

- render-stage timings can be inspected without changing deterministic render identity;
- incremental path timing is directly associated with the path report that produced it;
- structural counters give context to local timing samples;
- benchmark inputs and scenario semantics live in the repository and can evolve under review;
- local measurements remain diagnostic and must not be described as cross-browser or cross-machine performance claims;
- allocator-backed memory observability remains explicit future work rather than an R0 estimate.
