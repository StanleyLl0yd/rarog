# R6 — Compat backlog

Status: **in progress**.

Tracking issue: #329.

Canonical R6 starting baseline: `bd463d919190c4c49273b152ee27d852d0ab1e3d`.

R6 converts compatibility claims into reproducible evidence. It must not improve reported results by shrinking denominators, hiding failures, conflating synthetic fixtures with real runs, or treating unmeasured scope as passing.

## A. WPT measurement and dashboard

- [x] Deterministic `wptreport` ingestion, canonical dashboard output and synthetic regression coverage (#330).
- [x] Pin an upstream WPT revision and executable selected test manifest for the first real Rarog WPT run.
- [x] Produce and version the first non-synthetic Rarog WPT dashboard from actual execution.
- [x] Define update/comparison policy so regressions and newly measured scope remain distinguishable.

## B. Real-Web corpus

- [ ] Define a versioned real-Web scenario/corpus manifest with exact inputs and external-dependency policy.
- [ ] Add deterministic result capture that separates engine failures from unavailable/changed external services.
- [ ] Establish the first measured corpus baseline without converting it into a general-Web compatibility claim.

## C. Signed compatibility profiles

- [ ] Define the canonical compatibility profile payload from already-versioned evidence.
- [ ] Make profile generation reproducible and content-addressed.
- [ ] Add signing only after canonical payload reproduction is verified independently.

## D. High-priority Web app scenarios

- [ ] Select high-priority scenarios using explicit user-visible outcomes and bounded scope.
- [ ] Record deterministic setup, actions, expected observations and failure classification.
- [ ] Keep site-specific compatibility behavior out of standards-engine code unless a standards defect is independently demonstrated.

## E. WebDriver + BiDi qualification

- [ ] Define the minimum driver/session lifecycle needed to execute compatibility scenarios.
- [ ] Add WebDriver qualification evidence.
- [ ] Add WebDriver BiDi qualification evidence.
- [ ] Keep protocol presence distinct from conformance/coverage claims.

## F. Windows 10/11 real-machine compatibility

- [ ] Define machine/OS/build/hardware evidence metadata.
- [ ] Add Windows 10 real-machine compatibility runs.
- [ ] Add Windows 11 real-machine compatibility runs.
- [ ] Keep CI/VM evidence explicitly separate from real-machine evidence.

## G. R6 exit

- [ ] Add a dedicated deterministic R6 exit gate over selected compatibility evidence contracts.
- [ ] Reconcile README, Architecture, ADRs and roadmap with measured evidence only.
- [ ] Run a final repository-wide correctness/security/evidence-integrity review.
- [ ] Verify exact-head CI/Security/RustSec/CodeQL/Repository Full Audit.
- [ ] Verify the merged `main` matrix before closing #329.

## Stop boundary

R6 is compatibility qualification. Do not begin R7 stable View C ABI/binding work or R8 browser-product work as part of this milestone.
