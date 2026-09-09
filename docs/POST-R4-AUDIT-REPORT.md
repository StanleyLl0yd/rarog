# Post-R4 repository-wide audit report

Status: **complete; final merge is pending exact-head verification of this report-only closure update**.

This report records the mandatory full repository-wide audit/refactor/code review after R4 — Sky and before any R5 work. R5 is explicitly outside scope.

## Scope and endpoints

Audit base: `0dea3dc1b0af7a70b15b77a0fc33bcc3517cf81b`.

Verified implementation endpoint before this report: `1dd92fbc4c0bda72f560b0debaed221852ba0229`.

Tracking: issue #258, PR #259, branch `refactor/post-r4-full-audit`.

The first repository-wide pass covered the entire tracked repository:

- all 47 production Rust source files;
- all integration tests, examples and fuzz targets;
- all workspace/fuzz manifests and lockfiles;
- CI, security configuration and scripts;
- all 142 Markdown files, including architecture, milestone and ADR history.

The mandatory second pass re-reviewed all production source. Security-sensitive Host/Site process isolation, broker/capability, IPC/wire, Windows process/IPC, Host, Fetch, navigation and URL boundaries received the deeper pass first; the remaining compositor, CSS, DOM, events, HTML, layout, paint, platform, resources, scheduler, script/SpiderMonkey, scroll, shell, text/OpenType, types and WebIDL crates were then completed.

No production area remains unreviewed.

## Findings fixed

### Revoked Network backend cleanup retained outside the operation budget

Revoked Network tickets could previously leave backend work requiring cancellation while no longer consuming the Host operation budget. Repeated cleanup failure could therefore weaken the intended bounded-resource invariant.

The Host now quarantines revoked backend tickets and continues accounting them against the operation budget until backend cancellation succeeds. Cleanup failure retains the quarantine entry and budget charge; a regression test covers repeated failure/retry behavior.

Relevant implementation sequence:

- `ddb51e3` — bounded revoked Network backend cleanup;
- `ec2b244` — documented cleanup/accounting contract;
- `746ad6e` — cleanup-failure regression.

### Bounded CSS parsers allocated temporary unbounded token vectors

Three CSS parsing paths accepted only bounded arities but first collected every whitespace token into a temporary `Vec`:

- explicit Grid tracks: maximum 8;
- `gap` shorthand: maximum 2;
- edge-size shorthand: maximum 4.

On hostile declaration values this created avoidable O(n) temporary allocation before rejection. `1dd92fb` replaces those vectors with fixed storage/short iterators and fails closed immediately when the supported arity is exceeded. A regression covers oversized Grid, edge and gap inputs.

### Audit/verification gaps

The audit also closed verification and maintainability gaps without changing R4 behavior:

- added a dedicated pinned Post-R4 audit workflow and deterministic repository scanner;
- separated production source from inline test scanning;
- enforces zero production `unwrap()`, exact reviewed production `panic!` policy, and confinement of `unsafe` to the two explicit native/runtime boundaries;
- added hostile IPC wire-frame fuzz coverage and locked/documented the seventh fuzz target;
- restored unique current ADR numbering and reconciled post-R4 architecture/security/dependency/exit documentation;
- corrected stale R4 dependency and trust-boundary descriptions;
- repaired formatting-only CI regressions found by Windows rustfmt verification.

## Second-pass conclusions

No additional actionable security/correctness finding was confirmed after the two fixes above.

Important reviewed invariants include:

- web-facing DOM work is bounded before recursive layout traversal by `max_dom_nodes = 65_536` and `max_dom_depth = 512`; depth calculation itself is iterative;
- event listener registrations, scheduler queues, scroll nodes, image resources/decodes, source bytes/rooted script values, font faces/font bytes, framebuffer pixels and explicit CSS Grid tracks have explicit caps;
- Grid placement uses checked span arithmetic before indexing and rejects non-finite/overflowed geometry;
- framebuffer construction uses checked pixel multiplication and caps allocations at 67,108,864 pixels;
- malformed externally constructed display-list state is rejected by validated construction; internal raster stack `expect` sites remain behind private balanced-list invariants;
- SpiderMonkey keeps one localized FFI `unsafe` operation with an explicit JSContext/global-class/options lifetime invariant; its crate retains the dedicated unsafe lint boundary;
- WebIDL normalization rejects unsupported constructs instead of silently dropping semantics.

## Intentionally unchanged

The following reviewed candidates were preserved because changing them in this post-R4 gate would either alter a valid contract or solve a path that is not reachable from the current web trust boundary:

- standards HTML TreeSink invariant `expect` sites and the single exact reviewed `panic!` remain internal parser-adapter invariants;
- layout/fragment recursive helpers remain because the engine rejects documents deeper than 512 before those web-facing paths run;
- public layout/Grid APIs are not given a second project-wide node-count policy merely to guard arbitrary direct library misuse; the web path supplies the bounded inputs;
- `ScrollTree::remove_subtree` remains its simple scan implementation: the current engine creates only the root scroll node, so the theoretical multi-node quadratic path is not currently engine-reachable;
- event-dispatch snapshots, owned compositor/frame handoffs and other ownership snapshots remain where removing them would change mutation/lifetime or thread/IPC semantics;
- public compatibility APIs and deliberate adapter/crate boundaries were not deleted merely because they have one current workspace caller or implementation;
- no R5 implementation or speculative R5 refactor was started.

These decisions are reviewed residual constraints, not claims that future milestones must preserve them forever.

## Dependencies, fuzzing and repository metrics

At the verified implementation endpoint:

- tracked Rust files: 97;
- production Rust files: 47;
- fuzz targets: 7;
- Markdown files: 142;
- GitHub workflow files: 5;
- Cargo manifests/lockfiles: 31.

Compared with the audit base, the implementation endpoint is 19 commits ahead / 0 behind and changes 20 files with 657 additions and 107 deletions. The dependency review did not justify speculative replacement of mature parser, Unicode/shaping, GPU, URL/PSL, Windows or SpiderMonkey adapters.

The new `ipc_wire` fuzz target complements HTML, CSS, render, URL, Fetch and WebIDL hostile-input coverage. CI compiles the fuzz targets; this audit does **not** claim a long-duration fuzz campaign.

## Verification

Exact verified code endpoint `1dd92fbc4c0bda72f560b0debaed221852ba0229` is green in every required workflow:

- CI run 34386810713 — success, including Windows primary, Linux portability, MSRV 1.85 and SpiderMonkey feature lanes;
- Security run 34386810712 — success;
- Security Audit / RustSec run 34386810738 — success;
- CodeQL run 34386810711 — success;
- Post-R4 Full Audit run 34386810657 — success.

The implementation/report head `de1efdeffb785f9c371ce7035520c4fdf2a29176` completed every required workflow successfully. This closure update changes documentation only; PR #259 must also be green on its new exact head before merge.

## Limitations

- GitHub Actions is the authoritative Windows/Linux/MSRV/SpiderMonkey execution environment.
- Fuzz targets are compiled in CI; no sustained fuzzing-duration claim is made.
- Static/code review and CodeQL/RustSec do not prove correctness or memory safety of third-party native dependencies.
- No controlled cross-endpoint performance benchmark was run, so no performance percentage is claimed.
- Developer-facing direct library APIs can accept shapes broader than the engine exposes; this audit distinguishes those explicit library contracts from hostile web input rather than adding speculative global caps.
- R5 behavior, APIs and architecture were intentionally not evaluated or implemented.

## Completion gate

- Repository-wide pass 1: complete.
- Mandatory production pass 2: complete.
- Tests/examples/fuzz review: complete.
- Manifests/dependencies review: complete.
- CI/security/scripts review: complete.
- Documentation/ADR review: complete.
- Full PR diff review: complete; all 21 changed files at `de1efde…` were reviewed against `0dea3dc…` with no additional actionable finding.
- Exact-head GitHub verification at `de1efde…`: complete; CI, Security, Security Audit/RustSec, CodeQL and Post-R4 Full Audit all succeeded.
- Obsolete branch proof/deletion: complete; `backup/navigation-context-authority-pre249` and `r4/navigation-site-transitions` were proven superseded and deleted, leaving only `main` and this audit branch before merge.
- Final report-only closure head verification: required before merge.
- PR #259 squash merge and issue #258 closure: required.
- R5: **do not start**.
