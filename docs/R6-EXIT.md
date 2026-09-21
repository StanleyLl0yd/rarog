# R6 — Compat exit gate

Status: **in progress — evidence infrastructure is beginning**.

Tracking issue: #329.

Starting canonical baseline: `bd463d919190c4c49273b152ee27d852d0ab1e3d`.

## Purpose

R6 succeeds only when compatibility statements are backed by reproducible, versioned evidence. A feature being implemented is not itself compatibility evidence, and a synthetic/tooling fixture is never a substitute for an executed standards or real-Web test.

## Evidence invariants

Every publishable R6 evidence set must identify:

- the exact Rarog commit;
- the exact upstream WPT/corpus/profile revision or content digest;
- the measured platform/environment;
- the exact selected/measured scope;
- observed failures/timeouts/crashes as well as successes;
- whether expectation metadata was present;
- enough canonical machine-readable data to regenerate the human-readable summary.

Evidence tooling must fail closed on malformed/ambiguous records. Missing expectation metadata remains **unknown**, not implicitly expected or passing.

No dashboard may infer a result for a test that is absent from its input reports. Directory names, focus manifests and selected scopes are not denominators unless every member of that denominator is explicitly enumerated and measured.

## Required milestone evidence

Before R6 can close:

1. **WPT**
   - deterministic report ingestion/dashboard contract;
   - pinned upstream revision and exact executable selection;
   - at least one real, non-synthetic Rarog WPT evidence set;
   - explicit regression/update policy.
2. **Real-Web**
   - versioned corpus/scenario contract;
   - reproducible result capture with external-dependency classification;
   - measured baseline.
3. **Compatibility profiles**
   - canonical reproducible profile payload;
   - independent reproduction check;
   - signature over the canonical payload.
4. **High-priority scenarios**
   - explicit action/outcome scenarios with measured results.
5. **WebDriver/BiDi**
   - protocol/session evidence at the selected R6 scope.
6. **Windows**
   - explicitly identified Windows 10 and Windows 11 real-machine runs, kept distinct from hosted CI/VM evidence.
7. **Repository gates**
   - R0–R5 gates remain green;
   - R6 exit gate green on required platforms;
   - exact-head and merged-main CI/Security/RustSec/CodeQL/Repository Full Audit green.

## Current WPT slices

Issue #330 added deterministic WPT `wptreport` normalization/dashboard tooling. Its synthetic fixture exists only to verify tooling behavior and does **not** satisfy the required real WPT evidence item above.

Issue #332 pins an exact upstream WPT revision and a small file-level candidate selection with Git blob identities plus a local-checkout verifier. This establishes a reviewable denominator for the first real run; it also does **not** satisfy the real WPT evidence item until the selected tests are actually executed by Rarog and produce a non-synthetic report.

## Non-goals

R6 does not claim browser readiness, complete WPT conformance, general-Web compatibility, release-quality security/sandboxing, R7 embedding ABI stability or R8 reference-browser readiness.
