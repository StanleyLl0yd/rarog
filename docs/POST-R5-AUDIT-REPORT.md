# Post-R5 repository-wide audit report

This report records the mandatory full repository-wide audit/refactor/code review after completed R5 — Web and before any R6 work.

Tracking: issue #327, PR #328, branch `refactor/post-r5-full-audit`.

Audit base: `6cf38bf476c6529980025f92af96af4fe8495987`.

Verified implementation endpoint before this report: `b33986a8bb297e8891f413e3034a7ea84b14e4f4`.

The audit is complete when this report-bearing PR head also passes the exact required verification matrix and PR #328 is squash-merged. R6 remains outside scope.

## Scope and method

The first pass inspected the complete tracked repository rather than only R5 diffs:

- all 80 production Rust files across all 38 workspace crates;
- inline/unit tests, integration tests, examples and fuzz targets;
- all Cargo manifests and both committed lockfiles;
- all six GitHub workflows, Dependabot configuration and repository/security scripts;
- Windows-native/COM/UI Automation and SpiderMonkey unsafe boundaries;
- assets/resources and their convention/build references;
- architecture, roadmap, milestone, security, dependency and ADR documentation;
- open dependency-maintenance PRs and active repository ruleset state.

The mandatory second pass re-reviewed all 80 production Rust files on the refactored tree using an independent structural pass over allocations/collections, copies, checked/wrapping arithmetic, retained queues/maps, filesystem/process/native calls and unsafe/invariant sites. R5 Storage, Workers/Service Workers, WebSocket, media, Canvas/WebGL and accessibility/native boundaries received the deeper manual pass because they were added after the previous post-R4 full audit. Unchanged R0-R4 code was re-scanned and its residual invariants were checked against the completed post-R4 audit rather than changed merely for churn.

No production area remains unreviewed.

## Removed

- Removed the nonexistent `dependencies` label request from Dependabot configuration. Dependabot had been reporting that the configured label did not exist on every generated dependency PR.
- Removed the transient `Option -> iterator -> Vec` allocation from exact-scope Service Worker registration updates. At most one installing version can occupy that slot, so the vector represented no useful state.
- Removed milestone-specific active audit naming: the live `post-r4-audit.yml` / `post_r4_audit.py` names were replaced by durable repository-wide names. The historical post-R4 report remains historical evidence.
- No production source file, public API, asset or supported behavior was deleted solely to reduce line count.

## Consolidated

- The repository policy scanner is now the durable `scripts/repository_audit.py` and its workflow is `Repository Full Audit`.
- The audit workflow now runs on pushes to `main` as well as pull requests and manual dispatch, so merged-main repository policy evidence is produced continuously rather than only during the old post-R4 gate.
- Windows UI Automation attachment ownership is tied to the HWND itself through one private per-window property instead of introducing a process-global HWND registry.

## Simplified and hardened

### Windows UI Automation subclass lifetime

The R5 UIA bridge originally passed an `Arc::as_ptr()` raw address as `SetWindowSubclass` reference data while the wrapper remained the only strong owner. The wrapper `Drop` path ignored subclass-removal failure, so a window/wrapper teardown ordering outside the shell happy path could leave callback reference data pointing at released storage.

The bridge now:

- allocates an explicit callback-owned `SubclassContext` containing its own strong `Arc<UiaShared>`;
- retains that context until successful explicit subclass removal or terminal `WM_NCDESTROY`;
- invalidates retained provider/action state before wrapper teardown;
- rejects installation from a thread other than the HWND owner;
- rejects duplicate Rarog attachment using a private per-HWND property rather than a process-global registry;
- removes the property and callback context together on successful same-thread removal or terminal window teardown;
- leaves an inert callback-owned context in place when wrapper destruction occurs on another thread, avoiding forbidden cross-thread subclass-helper calls;
- scopes `CoInitializeEx`/`CoUninitialize` to concrete UIA entry operations with a same-thread RAII guard instead of coupling COM apartment balancing to a movable object lifetime.

An intermediate attempt to query `GetWindowSubclass` was rejected by Windows CI because importing that symbol made the native test executable fail to load with `STATUS_ENTRYPOINT_NOT_FOUND`; the final implementation does not depend on that entry point.

### Service Worker replacement

Exact-scope registration update now handles its single optional installing version directly. A regression test proves that a new update discards exactly the previous installing version, preserves the registration identity and leaves the replacement in the Installing slot.

### Dependency documentation

The `wgpu` pin rationale was corrected. Dependabot #320 demonstrates that `wgpu` 30 is not simply blocked by the Rust 1.85 compiler: Rarog reaches adapter compilation and then fails on concrete `wgpu` API changes. The documentation now describes 26.0.1 as the verified adapter/API baseline and requires an explicit GPU adapter migration for a future major upgrade.

## Fuzzing and hostile persisted input

R5 introduced a bounded binary Storage checkpoint restore parser after the post-R4 seven-target fuzz set was established.

A new `storage_checkpoint` fuzz target now feeds arbitrary bytes into `restore_storage_checkpoint` using deliberately small explicit Storage and checkpoint limits. The target exercises malformed persisted state while preserving bounded allocation and atomic candidate-state semantics.

The fuzz package remains outside the product workspace. Only local `rarog-process` and `rarog-storage` path dependencies were added; no new crates.io production or fuzz dependency was introduced.

The fuzz target set is now eight:

1. HTML parse
2. CSS stylesheet
3. full HTML render
4. URL parse
5. bounded Fetch values
6. WebIDL parse
7. IPC wire frame
8. Storage checkpoint restore

## Dependencies

Every production and development manifest was reviewed.

No production dependency was added, removed or version-upgraded by this audit. Mature parser, URL/PSL, Unicode, shaping, GPU, Windows and SpiderMonkey dependencies remain behind their existing adapter boundaries.

The currently open Dependabot PRs were reviewed rather than merged opportunistically:

- #317 updates minor/patch Cargo dependencies; the workspace check succeeded, but the PR's fuzz lockfile is stale under `--locked`. It requires a coherent two-lockfile refresh before consideration.
- #318 `cssparser` 0.38 removes the `ParserInput` API used by the current private CSS adapter and therefore requires an explicit adapter migration.
- #319 `mozjs` 0.23.1 previously passed its isolated feature matrix but is now behind current `main`; it requires rebase/current-head verification and is unrelated to this behavior-preserving audit.
- #320 `wgpu` 30.0.1 changes several adapter APIs (`multiview_mask`, pipeline-layout binding types/fields and sampler filter types) and therefore requires an explicit compositor/GPU migration.
- #321 `font-test-data` 0.9.1 previously passed its PR matrix but is behind current `main`; it remains a separate test-data dependency update.
- #322 updates GitHub Actions and previously passed its PR matrix but is behind current `main`; it remains separate from the audit.

Keeping those updates separate avoids mixing compatibility migrations or stale lockfile refreshes into a repository-wide correctness/refactor gate.

## Legacy

- The historical `docs/POST-R4-AUDIT-REPORT.md` still described its already-merged closure as pending. It now records the actual final verified head and squash merge.
- The active repository scanner/workflow no longer encodes R4 in its name.
- No obsolete R0-R5 production compatibility branch or duplicate implementation was found that could be proven safe to delete.

## Intentionally unchanged

The second pass explicitly reviewed and preserved the following because removing or consolidating them would weaken behavior, atomicity or ownership clarity rather than reduce necessary complexity:

- the 65 production `expect` sites are internal validated/default/invariant boundaries; the repository scanner still reports them for review;
- the single production `panic!` remains the previously reviewed HTML TreeSink invariant;
- unsafe Rust remains confined to the two deliberate boundaries, `rarog-platform-windows-native` and `rarog-script-spidermonkey`, both with `unsafe_op_in_unsafe_fn = "deny"`;
- Worker subtree/owner snapshots are bounded by worker limits and permit safe mutation after traversal;
- Graphics-adapter cleanup snapshots are bounded by resource limits and avoid mutating maps while iterating them;
- Storage checkpoint origin/key vectors are needed for deterministic canonical ordering and use fallible reservations;
- Canvas output copies are required to transfer owned decoded pixels and use fallible exact reservation after pixel bounds are established;
- Accessibility preview-state cloning preserves atomic snapshot/event publication on refresh failure;
- media stream snapshots preserve exact ownership validation and are bounded by media limits;
- event/compositor/render ownership snapshots remain where removing them would change mutation, thread or retained-frame semantics;
- large R0-R4 modules were not split solely because of file size; no split was shown to reduce total responsibility, coupling or regression risk in this audit;
- branding assets, the HTML WPT fixture, bootstrap HTML and R1 focus marker remain active repository/build/test inputs.

## Repository and security review

The final tree preserves:

- immutable full-SHA GitHub Action pins;
- digest-pinned security container usage;
- workflow-level empty permissions with job-local read permissions;
- no `pull_request_target`;
- locked root and fuzz dependency resolution;
- RustSec coverage of both lockfiles;
- Security, Semgrep, Gitleaks, Dependency Review and CodeQL workflows;
- workspace `unsafe_code = "forbid"` outside the two reviewed native/runtime exceptions.

The active ruleset requires `Verify`, `Security Gate`, `RustSec`, `Analyze Rust` and `Analyze GitHub Actions`, requires linear signed squash merges, and resolves review threads. `Repository Full Audit` is not currently a required ruleset status; changing repository rules requires administration permission not available to the connected GitHub integration. The workflow still runs on every PR and merged-main push and is treated as required evidence by this audit.

## Verification

Verified implementation endpoint `b33986a8bb297e8891f413e3034a7ea84b14e4f4` completed the full matrix successfully:

- CI #1226 — success, including Windows primary, Linux portability, Rust 1.85 MSRV, SpiderMonkey Windows/Linux, formatting, check, Clippy, complete workspace tests, R0/P1/R0.1/R1/R2/R3/R4/R5 exit gates and bootstrap render;
- Security #398 — success;
- Security Audit / RustSec #413 — success;
- CodeQL #398 — success;
- Repository Full Audit #13 — success, including repository policy scan, Cargo metadata, duplicate dependency inspection, doc tests, rustdoc warnings, all fuzz target compilation and CI supply-chain verification.

The report-bearing PR head must pass the same exact-head evidence before PR #328 leaves draft or merges.

## Limitations

- The audit compiles all eight fuzz targets but does not claim a sustained fuzz campaign.
- CI exercises the Windows-native code and exact UIA build/lifetime tests, but this audit does not claim assistive-technology compatibility certification or broad UIA pattern coverage.
- Static review, CodeQL, Semgrep and RustSec do not prove correctness or memory safety of third-party native dependencies.
- No controlled before/after performance benchmark was run, so no performance percentage is claimed.
- The GitHub integration can read the repository ruleset but cannot administer it; making `Repository Full Audit` a branch-protection-required status remains a repository-administration follow-up.
- Dependency upgrade PRs intentionally remain separate where they require API migration, coherent lockfile refresh or current-head revalidation.

## Before/after statistics

Using the same repository scanner/counting method:

| Metric | Audit base | Verified implementation |
| --- | ---: | ---: |
| Tracked files | 369 | 370 |
| Rust files | 137 | 138 |
| Production Rust files | 80 | 80 |
| Tracked Rust lines | 79,887 | 80,109 |
| Markdown files | 171 | 171 |
| GitHub workflow files | 6 | 6 |
| Cargo manifests | 40 | 40 |
| Committed Cargo lockfiles | 2 | 2 |
| Fuzz targets | 7 | 8 |

The implementation adds one fuzz target and hardening/regression code; it does not increase the production file count or production dependency graph. This report itself adds one Markdown file after the verified implementation endpoint.

## Completion gate

- Repository-wide pass 1: complete.
- Production pass 2: complete, 80/80 production Rust files.
- Tests/integration/examples/fuzz review: complete.
- Assets/resources review: complete.
- Manifests/dependencies review: complete.
- CI/security/repository tooling review: complete.
- Documentation/ADR review: complete.
- Ruleset/branch/dependency-PR hygiene review: complete.
- Full implementation diff review: complete.
- Verified implementation matrix: complete at `b33986a8bb297e8891f413e3034a7ea84b14e4f4`.
- Report-bearing exact-head verification: required before merge.
- PR #328 squash merge / issue #327 closure: required after exact-head verification.
- R6: **do not start as part of this audit**.
