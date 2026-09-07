# Pre-R4 repository audit and stabilization gate

Status: **pending**.

R4 implementation is blocked while this status is pending.

## Objective

After R3 closes and before any R4 Host/Site process, IPC, sandbox, capability-broker, site-isolation or crash-recovery work begins, audit and refactor the complete repository.

The mandatory execution protocol is `agent/AUDIT_REFACTOR.md`. Rarog-specific invariants in `../AGENTS.md` remain authoritative.

No new product functionality is added during this phase.

## Required sequence

- [x] Capture the post-R3 baseline: repository structure, crate/dependency graph, test/CI state and practical size/count metrics (`PRE-R4-AUDIT-REPORT.md`).
- [x] Complete repository-wide audit pass 1.
- [x] Apply high-confidence behavior-preserving deletion, consolidation, simplification and dependency cleanup in focused PRs.
- [x] Run the relevant full verification after every meaningful refactor group.
- [x] Complete mandatory repository-wide audit pass 2 over the already-refactored tree.
- [x] Resolve or explicitly record every remaining candidate that cannot be proven safe.
- [x] Reconcile architecture docs, roadmap, ADRs, README, AGENTS and CI with the resulting implementation.
- [ ] Run the final stabilization suite on Windows-primary, Linux portability, MSRV and SpiderMonkey gates.
- [ ] Produce the final audit report with before/after metrics, changes made, preserved deferrals and any accepted residual debt.
- [ ] Mark this gate complete before opening the first R4 implementation PR.

## Audit emphasis for the R3 → R4 boundary

In addition to the general protocol, inspect these areas closely before process boundaries make them more expensive to change:

- crate ownership and dependency direction;
- DOM/CSS/layout/paint/compositor/platform API leakage;
- duplicated state and conversions across retained rendering paths;
- frame scheduling, compositor worker ownership and backpressure;
- GPU/resource/surface lifecycle and failure recovery boundaries;
- async resource queue ownership and limits;
- scroll identity, damage and viewport translation state;
- Grid/Flex compatibility paths and stale bounded-slice scaffolding;
- unnecessary clones, allocations, intermediate vectors and repeated validation in hot paths;
- public API surface that would become an IPC contract if left untouched;
- `unsafe` policy and platform isolation;
- dependency overlap and obsolete feature flags;
- CI duplication, stale tests, stale ADRs/TODOs and obsolete compatibility branches.

## Exit condition

This document may change to `Status: **complete**` only when every checklist item above is checked and the final stabilization CI is green.

Only then may R4 begin.
