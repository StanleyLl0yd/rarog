# Pre-R4 audit report

Status: **complete**.

This report records the mandatory repository-wide audit/refactor between R3 — Wings and R4 — Sky. Final stabilization is green and the gate is complete in this finalization branch; R4 remains blocked until the completed gate/report are merged to `main`.

## Scope and endpoints

Canonical baseline commit: `a00a5e567253d60ae22c544ad82fed7ee1db78ed`.

Implementation/documentation endpoint before this final report: `a73833a8d37226996bdd1b973224f44c4e187218`.

The audit followed `docs/agent/AUDIT_REFACTOR.md`, `AGENTS.md` and issue #111. Both repository-wide passes reviewed the current production tree, tests/test utilities, examples, fuzz targets, manifests, workflows, dependency state, platform adapters, architecture/roadmap/ADR contracts and project-facing resources. Every one of the 87 tracked Rust files was read in the second pass: 39 production source files and 48 integration-test/example/fuzz files.

Audit work was delivered through focused PRs #203, #206–#219. Branding-only PRs #204/#205 occurred after the chosen baseline but are not audit changes; their byte impact is separated in the metrics section.

No R4 Host/Site process, IPC, sandbox, capability-broker, site-isolation or crash-recovery implementation was started during this audit.

## Removed

The audit removed only work whose lack of value could be demonstrated without changing the public/behavioral contract:

- redundant explicit Cargo default `src/lib.rs` declarations;
- the empty `[dependencies]` table in `rarog-script`;
- two full display-list snapshots from asynchronous image-refresh handling;
- a temporary heap `Vec<GridTrackSize>` in the bounded Grid-size constructor;
- a second `String` allocation when canonicalizing standard Fetch methods;
- fallible image-decode/engine completion `expect` paths that could use existing error/optional contracts instead;
- Grid placement invariant panics where the public API already has an appropriate error path;
- repeated DOM mutation `unreachable!` paths where the public API already exposes `NotElement` / `NotText`.

No production crate, dependency, public API, test suite, fuzz target or project resource was proven obsolete enough to delete.

## Consolidated

- equivalent CSS Grid column/row track conversion and fallback construction now use exhaustive shared helpers;
- paint traversal stores cumulative transform state per stack depth instead of replaying the active transform stack for every rectangle;
- Grid fixed-size construction now writes directly into its bounded inline representation;
- image-refresh detection/damage derivation uses existing retained state instead of constructing throwaway snapshots.

The deeper Grid/Flex sizing algorithms were deliberately not generalized merely to reduce visual repetition.

## Simplified and hardened

- viewport validation now rejects NaN and infinity at the engine boundary before layout/paint/framebuffer work;
- Windows normalized input retention is bounded to 4,096 queued events and default retained text is bounded to 1 MiB UTF-8; queue saturation and oversized IME input fail through existing contracts;
- paint validation rejects non-finite cumulative transforms and projected rectangle overflow;
- Fetch standard-method canonicalization mutates the already-owned input string in place while extension-method spelling remains unchanged;
- image decode completion and Grid/DOM public fallible paths no longer rely on avoidable invariant panics;
- the Windows SpiderMonkey feature lane now runs Clippy with warnings denied, matching the Linux feature lane;
- CI gained a read-only, pinned RustSec advisory workflow using `cargo-audit 0.22.2`.

No performance-leadership claim is made. These changes remove evident repeated work/allocations or strengthen bounded failure behavior; benchmark claims remain measurement-dependent.

## Dependencies

Every direct workspace and fuzz dependency was reviewed against current source, target-specific integration, build/test/fuzz use and adapter ownership.

No dependency was removed or replaced because none was proven unused or duplicate-purpose without increasing custom code, architectural leakage or maintenance risk.

The dependency graph is unchanged from baseline:

- 372 `Cargo.lock` package entries;
- 350 registry/Git package entries;
- 22 workspace package entries;
- 320 unique external package names.

The selected adapters and exact manifest versions are now reconciled in `docs/DEPENDENCIES.md`, including `html5ever`, `cssparser`, `weedle2`, `url`/`psl`, HarfRust, SpiderMonkey, `wgpu`, `font-kit` and `clipboard-win`.

The SpiderMonkey adapter remains the intentional narrow exception to workspace `unsafe_code = "forbid"`; the production scan found the single required `unsafe` operation inside that adapter and no vendor/OS API leakage into Web-facing core crates.

## Legacy and documentation drift

The audit found documentation that still described pre-R3 state after R3 had completed.

Reconciliation updated README, CONTRIBUTING, ARCHITECTURE, DEPENDENCIES and the R3 exit status to match the implemented R0–R3 tree and current CI.

Two historical ADR identifiers had been reused:

- the later DOM/script-wrapper lifetime decision is now canonical ADR 0100;
- the later standards HTML parser adapter decision is now canonical ADR 0101.

The old 0013/0014 paths are retained as short link-preserving aliases rather than breaking existing references.

`docs/AUDIT-REPORT-2026-09-03.md` remains intentionally as a historical #81 audit record and is explicitly distinguished from this R3 → R4 audit.

Public compatibility aliases and historical milestone records were not deleted merely because current internal code no longer depends on them.

## Intentionally unchanged

The following candidates were reviewed and preserved because a safe simplification could not be proven without changing ownership, lifetime or public/architectural contracts:

- compositor worker submission still owns a frame snapshot; eliminating those owned copies requires an explicit shared-snapshot/IPC ownership redesign;
- `FetchRequest::network_request()` still creates an owned network handoff snapshot rather than exposing request lifetimes across the capability boundary;
- event dispatch snapshots registration IDs and owns `current_target`; removing that state would change mutation-during-dispatch or public lifetime semantics;
- bounded Fetch header lookup remains linear; an additional index would introduce duplicated mutable state for a deliberately bounded list;
- scheduler task/microtask queues and `active` identity remain separate because payload ownership leaves the queue while completion identity must stay retained;
- public APIs with no current workspace caller remain public observable surface and were not classified as dead code from text-reference counts;
- standards HTML TreeSink and layout/paint internal structural `expect`/`unreachable!` sites remain where they encode construction invariants and replacing them would add artificial public error semantics;
- deliberate crate/adapter boundaries remain even when only one implementation currently exists, because they isolate platform, GPU, parser, shaping, networking or SpiderMonkey semantics;
- the dedicated SpiderMonkey lint boundary remains per ADR-0052.

These are residual design constraints, not authorization to carry them unchanged through R4 if process/IPC design provides a demonstrated simpler ownership model.

## Tests, fuzzing and static review

The second pass read all 48 non-production Rust files in addition to all production source.

At the final source endpoint:

- tracked Rust files: 87;
- Rust lines by the baseline counting method: 43,889;
- `#[test]` occurrences: 687;
- literal `TODO` occurrences: 0;
- literal `FIXME` occurrences: 0;
- `#[ignore]` occurrences in integration/example/fuzz review: 0.

The six fuzz targets cover HTML parsing, CSS stylesheet parsing, render construction, URL parsing/resolution, Fetch values and WebIDL parsing. CI compiles all fuzz targets on Linux; this audit did not claim a long-running fuzz campaign.

The production boundary scan also confirmed:

- `wgpu` types are confined to `rarog-compositor-wgpu` and the Windows GPU adapter;
- `font-kit` / `clipboard-win` are confined to the Windows platform crate;
- `mozjs` is confined to the SpiderMonkey adapter;
- no Windows/GPU/SpiderMonkey dependency types leak into DOM, CSS, layout, paint or engine-core public paths.

## Verification

Baseline GitHub Actions run `34109430071` was fully green on the exact baseline commit.

Every accepted code/refactor group was merged only after its required pull-request CI passed. The audit PR series includes the viewport, manifest, Grid, RustSec, paint-transform, Windows-input, image-refresh, panic-removal, allocation, Fetch, DOM, SpiderMonkey-Windows-Clippy and documentation-reconciliation changes.

Documentation reconciliation PR #219 passed CI run `34122347554` with:

- Windows primary — success;
- Linux portability — success;
- MSRV 1.85 — success;
- SpiderMonkey Windows — success;
- SpiderMonkey Linux — success.

The final post-reconciliation `main` stabilization run `34122878456` on `a73833a8d37226996bdd1b973224f44c4e187218` completed successfully with Windows primary, Linux portability, MSRV 1.85, SpiderMonkey Windows and SpiderMonkey Linux all green.

The closure PR also advances the R3 exit integration assertion from “pre-R4 pending” to the durable completed-transition contract: R3 remains complete, the pre-R4 gate is complete, and neither document may contain unchecked gate items.

RustSec run `34116654609` completed successfully on commit `73e756b9136d018702c27e2741bb1803a4cd6a6d`, after the manifest cleanup and introduction of the audit workflow. No dependency declarations or `Cargo.lock` contents changed after that dependency-state commit, so it audits the same dependency graph recorded at the final endpoint.

The Linux portability lane includes fuzz-target compilation. The Windows/Linux lanes run workspace checks/tests, milestone/correctness gates and bootstrap rendering; the separate feature lanes exercise SpiderMonkey; MSRV is pinned to Rust 1.85.

## Limitations

- GitHub Actions is the authoritative Windows/Linux/MSRV/SpiderMonkey execution environment. The connector-based audit did not rely on an independent local network clone.
- No macOS CI lane exists because macOS is not yet an active target.
- Fuzz targets were compiled by CI, but no long-duration fuzz campaign is claimed.
- No production binary-size comparison is reported: the workspace is primarily an engine/library plus bootstrap shell, and no stable release artifact was produced at both endpoints with an identical artifact-measurement procedure.
- No benchmark improvement percentage is claimed; the audit removed obvious repeated work but did not run a controlled cross-endpoint performance benchmark suite.
- Candidates requiring ownership/public-API/process-boundary redesign were preserved for explicit R4 architectural work rather than changed speculatively.
- Numerous historical topic branch refs predate the current cleanup policy and remain outside the audited `main` tree. Repository `delete_branch_on_merge` is enabled for future merged topics, but the available GitHub connector exposes no delete-ref mutation, so retroactive branch-ref deletion is an administrative cleanup item rather than a claimed audit change.

## Before/after statistics

Repository/tree metrics use recursive Git blob sizes. Rust lines use UTF-8 text split on newline for every tracked `.rs` file. Dependency metrics parse the same `Cargo.lock` package/source fields at both endpoints.

| Metric | Baseline | Final |
| --- | ---: | ---: |
| Repository files (Git blobs) | 249 | 253 |
| Repository blob bytes | 1,977,666 | 3,869,498 |
| Repository bytes excluding canonical repository hero master | 1,974,310 | 2,002,333 |
| Workspace crates | 22 | 22 |
| Rust files | 87 | 87 |
| Rust blob bytes | 1,452,714 | 1,461,930 |
| Rust lines, including source/tests/examples/fuzz | 43,620 | 43,889 |
| `#[test]` occurrences | 682 | 687 |
| literal `TODO` occurrences | 0 | 0 |
| literal `FIXME` occurrences | 0 | 0 |
| Markdown files | 126 | 129 |
| Markdown blob bytes | 401,225 | 419,288 |
| TOML files | 25 | 25 |
| TOML blob bytes | 9,340 | 9,143 |
| Lockfile package entries | 372 | 372 |
| Registry/Git package entries | 350 | 350 |
| Workspace package entries | 22 | 22 |
| Unique external package names | 320 | 320 |

The raw repository-byte increase must not be interpreted as code growth. After the baseline, branding PRs #204/#205 replaced the 3,356-byte WebP repository hero with the exact approved 1,867,165-byte original PNG. That branding-only change accounts for nearly all raw byte growth. The adjusted row excludes only the canonical repository hero master at each endpoint so the audit/documentation change can be compared without that unrelated asset replacement.

The Rust source/test total increased by 269 lines and five test occurrences while adding boundary regressions, hardening and the completed R3 → pre-R4 transition assertion. TOML bytes decreased by 197 through manifest cleanup. The dependency graph did not grow.

## Completion

- Baseline capture: complete.
- Repository-wide audit pass 1: complete.
- High-confidence behavior-preserving cleanup/hardening: complete.
- Mandatory repository-wide audit pass 2: complete.
- Remaining candidates resolved or explicitly preserved: complete.
- Architecture/roadmap/ADR/README/AGENTS/CI reconciliation: complete.
- Final stabilization: complete; run `34122878456` is green.
- Final report: complete.
- R4 transition gate: complete in this finalization branch; no R4 implementation has started.
