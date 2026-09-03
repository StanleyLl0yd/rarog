# Repository-wide audit report — 2026-09-03

## Status and scope

This report records the repository-wide behavior-preserving audit/refactor tracked by #81.

- audit base: `a66250daa65df36d6ca6e634d27ac36189b6d045`;
- implementation/documentation endpoint before this report: `a13cb3119e61daa3170af426a4a492e2e255e30a`;
- implementation PRs: #82, #83, #84, #85, #86, #87, #88, #89 and #90;
- public architecture contracts, Windows-first/Linux-portable behavior, Rust 1.85 MSRV and `unsafe_code = "forbid"` were preserved.

The audit covered production crates, tests and test utilities, workspace/build configuration, CI, fuzz integration, direct dependencies, security/advisory state, documentation, platform adapters and the R0/P1/R0.1/R1 verification boundaries. A mandatory second pass was performed after the main refactor slices.

## Removed

The audit removed proven unnecessary work rather than deleting functionality:

- the hostile-input UTF-8 byte-slicing panic path in six-digit CSS hex-color parsing;
- a temporary `BTreeSet` allocation from each required-class selector match;
- per-property cascade candidate-vector clones before sorting;
- recursive CSS style-source and invalidation subtree walks where iterative DFS provides the same traversal semantics;
- duplicate image-resource map lookups and avoidable ready-state invariant `expect` paths;
- a second OpenType cluster-boundary validation scan;
- a second construction of the same fragmenting-inline stream;
- a second scan of the same display-list structural prefix during retained-range replacement;
- unconditional cloning of an unchanged `StyleSet` during ordinary incremental updates;
- repeated full-source scans for each recoverable HTML parser diagnostic;
- repeated full grapheme-boundary scans for every shaping run;
- three copies of the same framebuffer `Rect` to integer pixel-bound rounding/clamping formula;
- the engine's internal reliance on the legacy standards-parser compatibility alias.

No tracked production file, resource or dependency was deleted because the full review did not prove any such item unused without changing a public/architectural contract.

## Consolidated

- CSS-wide `inherit`/`initial` property copying now shares one property-copy implementation.
- Framebuffer image/fill/clear raster paths share one private pixel-bound conversion while retaining their distinct rendering loops.
- Existing canonical HTML parser entry points are used internally; compatibility aliases remain only as public compatibility surface.
- Repeated validation/traversal work was combined only where the resulting path is strictly smaller or cheaper and preserves the same failure semantics.

## Simplified and reduced runtime work

- Selector class checks no longer allocate a set for each match.
- Cascade resolution sorts the owned candidate vectors in place.
- CSS subtree processing uses bounded iterative traversal instead of recursive call chains.
- Image ready-state transitions perform one resource lookup after validation.
- Layout constructs the fragmenting inline stream once per candidate.
- Retained display replacement computes the incoming structural scope once and advances it across the affected range.
- Incremental rendering borrows the retained `StyleSet` when stylesheet sources are unchanged and only materializes ownership for a real stylesheet rebuild or full-rebuild fallback.
- HTML diagnostics build line-start offsets once, and only when diagnostics exist; valid input does not allocate that index.
- Shaping request segmentation restricts grapheme-boundary iteration to the current run using range partitioning.
- Framebuffer raster paths now use one rounding/clamping implementation, reducing semantic drift risk.

No benchmark or performance-leadership claim is made. These changes remove evident repeated work and allocations; the project still treats performance claims as measurement-dependent.

## Dependencies

All direct workspace and fuzz dependencies were reviewed against their current code, platform, build, test or fuzz use. No dependency was proven safely removable or duplicative enough to justify replacement.

`Cargo.toml`, crate manifests and `Cargo.lock` are unchanged by the audit. The locked dependency graph therefore did not change.

An explicit RustSec pass was run during the audit (`cargo-audit` run 33728150905) and completed successfully. It loaded 1,239 advisories and scanned 102 locked crate dependencies without reporting a vulnerability.

A fresh RustSec scan is also part of the final consolidated verification run 33733178679. **Final RustSec result: pending while this report draft is prepared; this line must be resolved before merge.**

## Legacy

The engine no longer calls the legacy `parse_standards*` HTML compatibility aliases internally; it uses the canonical standards-oriented parser API. The public aliases remain because removing them would be an intentional public API break rather than behavior-preserving cleanup.

No obsolete compatibility file or historical architectural layer was removed merely because it currently has one implementation. Platform, shaping, WebIDL and embedder interfaces were retained where they define real ownership/replaceability boundaries required by the project architecture.

## Intentionally unchanged

The second pass reviewed several candidates and deliberately preserved them:

- `rarog_dom::try_node` remains as a public compatibility alias; deletion would change the public API.
- The DOM generation `checked_add(...).expect(...)` remains an exhaustion invariant. Making every mutation API fallible for a practically unreachable `u64` exhaustion case would increase public complexity.
- HTML TreeSink internal `expect` paths are protected by the `html5ever` adapter contract and its own state transitions; they are not direct hostile-input indexing paths.
- Paint clip/opacity stack `expect` paths remain behind validated `DisplayList` construction and balanced internal builders. Exposing extra fallibility there would duplicate an already-enforced invariant.
- Retained display-list replacement keeps its candidate clone because the copy provides atomic rollback if validation fails.
- Embedder viewport/source/resource-budget validation remains intentionally duplicated at trust/allocation boundaries where it prevents oversized allocation or preserves a public error contract.
- Rarog-owned traits/adapters with one current production implementation were retained where they provide a deliberate platform/backend ownership seam rather than speculative abstraction.
- Mature third-party parsing, Unicode and shaping components were not replaced with local code solely to reduce dependency count.

## Documentation

The audit synchronized architecture, metrics, contributing/fuzz guidance and project status with the actual repository state:

- current retained text/structural/stylesheet/formatting invalidation paths;
- structural damage-aware replay rather than the obsolete full-frame fallback description;
- canonical HTML parser entry points and retained compatibility aliases;
- the existing R1 exit gate and CSS fuzz target;
- R2 — Flight as an active milestone, including the already-created Rarog-owned WebIDL IR/frontend boundary and `rarog-webidl` workspace crate.

## Verification

Every clean implementation PR (#82–#90) passed the repository's normal CI before merge, including Windows-primary, Linux-portability and Rust 1.85 MSRV coverage plus the applicable R0/P1/R0.1/R1 gates.

Post-#90 `main` CI run 33733075323 completed successfully at `a13cb3119e61daa3170af426a4a492e2e255e30a`.

The dedicated final verification run 33733178679 is based on that exact implementation commit plus only its temporary verification workflow. It runs:

- `cargo fmt --all -- --check`;
- `cargo check --workspace --all-targets`;
- strict `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`;
- fuzz-target compilation;
- R0, P1, R0.1 and R1 exit/correctness gates;
- focused paint-damage, CSS finite-length and DOM-invariant regressions;
- release workspace builds;
- bootstrap rendering;
- Rust 1.85 workspace and fuzz-target checks;
- a fresh `cargo audit` advisory scan.

At report-draft time, Windows final verification and MSRV 1.85 final verification are fully successful. Linux has passed formatting, workspace check, strict Clippy, all workspace tests, fuzz compilation, all exit/correctness gates, focused regressions, release build and bootstrap render; only installation/execution of the final `cargo-audit` remains in progress. The final report must not merge until that result is resolved.

## Before/after statistics

Using one Git compare from audit base `a66250daa65df36d6ca6e634d27ac36189b6d045` to implementation endpoint `a13cb3119e61daa3170af426a4a492e2e255e30a`:

- commits: 9 ahead / 0 behind;
- changed existing files: 14;
- production source files changed: 8;
- documentation/support files changed: 6;
- added lines: 272;
- deleted lines: 251;
- net diff: +21 lines;
- added tracked files: 0;
- deleted tracked files: 0;
- Cargo manifest/lock changes: 0.

These are Git diff statistics, not a claim about total repository source LOC. A matching whole-repository source-line baseline was not captured before the audit, so no total LOC before/after figure is invented.

## Limitations

- The isolated local runtime could not clone GitHub over external DNS, so a local whole-tree `rg` pass was unavailable. Repository inspection used GitHub file/tree access plus compilation, Clippy and test/fuzz gates.
- GitHub code search returned unreliable empty results even for known existing symbols during the second pass, so it was not treated as proof of absence. Candidates were validated against fetched source files, public/indirect contracts and compiler/lint results instead.
- No general-Web compatibility, performance, security-hardening or browser-readiness claim follows from this audit.
- No benchmark threshold or artifact-size baseline was available that would support a reliable before/after performance or binary-size claim.
- Code whose lack of use or redundancy could not be proven was intentionally preserved.

## Conclusion

The audit removed demonstrated panic/duplication/repeated-work paths, simplified hot-path ownership and traversal, synchronized documentation, reviewed dependencies and then re-read the already-refactored production crates. No further second-pass production change was accepted once the remaining candidates crossed into public API, invariant, rollback-safety or deliberate architectural-boundary territory.

The final repository state is intended to preserve current externally observable behavior while containing less unnecessary allocation, copying, traversal and duplicated implementation logic.
