# Pre-R4 audit report

Status: **in progress**.

This report records the repository-wide audit/refactor required between R3 and R4. R4 remains blocked by `PRE-R4-AUDIT.md` until the complete two-pass audit, behavior-preserving cleanup, final stabilization and this report are complete.

## Baseline

Canonical baseline commit: `a00a5e567253d60ae22c544ad82fed7ee1db78ed`.

This is the first `main` commit containing both the completed R3 exit gate and the canonical Rarog branding, so unrelated milestone/branding changes do not distort later before/after measurements.

### Repository structure

Measured from the recursive Git tree at the baseline commit:

| Metric | Baseline |
| --- | ---: |
| Repository files (Git blobs) | 249 |
| Repository blob bytes | 1,977,666 |
| Workspace crates | 22 |
| Rust files | 87 |
| Rust blob bytes | 1,452,714 |
| Markdown files | 126 |
| Markdown blob bytes | 401,225 |
| TOML files | 25 |
| TOML blob bytes | 9,340 |

The workspace members are:

- `rarog-types`
- `rarog-resources`
- `rarog-dom`
- `rarog-events`
- `rarog-html`
- `rarog-css`
- `rarog-layout`
- `rarog-text-opentype`
- `rarog-webidl`
- `rarog-url`
- `rarog-fetch`
- `rarog-script`
- `rarog-script-spidermonkey`
- `rarog-scheduler`
- `rarog-scroll`
- `rarog-paint`
- `rarog-compositor`
- `rarog-compositor-wgpu`
- `rarog-platform`
- `rarog-platform-windows`
- `rarog-engine`
- `rarog-shell`

The fuzz package is intentionally excluded from the workspace and checked separately by CI.

### Rust source/test counts

Every tracked `.rs` file was read from the baseline tree. The line-count method is UTF-8 text split on newline, applied identically to all Rust files and to be reused for the final comparison.

| Metric | Baseline |
| --- | ---: |
| Rust lines, including source/tests/examples/fuzz targets | 43,620 |
| `#[test]` occurrences | 682 |
| literal `TODO` occurrences | 0 |
| literal `FIXME` occurrences | 0 |

The `unwrap`, `expect` and `clone` scan is being used only to prioritize manual review. Their raw occurrence counts are not correctness findings and are not reduction targets.

### Dependency baseline

Parsed from the baseline `Cargo.lock` using the same package-block/source-field method that will be used at completion:

| Metric | Baseline |
| --- | ---: |
| Lockfile package entries | 372 |
| Registry/Git package entries | 350 |
| Workspace package entries | 22 |
| Unique external package names | 320 |

Every direct Cargo manifest is part of audit pass 1. Dependency removal requires proof that the dependency is unused or redundant; transitive lockfile size alone is not evidence for removal.

### Verification baseline

GitHub Actions run `34109430071` executed on the exact baseline commit and completed successfully.

Green jobs:

- Windows primary
- Linux portability
- MSRV 1.85
- SpiderMonkey Windows
- SpiderMonkey Linux

The Windows/Linux jobs include the R0, P1, R0.1, R1, R2 and R3 exit/correctness gates as configured after R3 closure. Linux also verifies fuzz-target compilation.

## Initial pass-1 observations

These are candidates, not automatic deletion instructions.

### High-confidence configuration noise

Several crates explicitly declare the Cargo default library path `src/lib.rs`. Those declarations do not alter build behavior and are candidates for deletion. `rarog-script` also has an empty `[dependencies]` table.

This is an appropriate first behavior-preserving cleanup group because it changes no crate boundary, API, dependency version or runtime code.

### Intentionally preserved SpiderMonkey lint boundary

`rarog-script-spidermonkey` does not inherit the workspace `unsafe_code = "forbid"` lint. This is intentional, not configuration drift.

ADR-0052 requires the dedicated adapter to contain the narrow JSAPI unsafe operation while ordinary engine crates retain the workspace prohibition. The adapter instead denies `unsafe_op_in_unsafe_fn`. The audit will preserve this boundary unless the FFI design itself changes through a separate reviewed architectural decision.

### Public APIs are not dead code merely because the workspace does not call them

Examples already reviewed include public Grid compatibility/constructor APIs with no internal production caller. They remain externally observable library surface and will not be removed solely on internal text-reference counts.

### R3 Grid duplication candidate

The production Grid integration contains repeated, equivalent conversion/projection logic for column and row track lists. This is a candidate for a small consolidation only if the helper form keeps enum handling exhaustive and makes the control flow objectively smaller. The deeper Grid sizing algorithms will not be generalized merely to reduce repetition.

## Method and limitations

The audit follows `agent/AUDIT_REFACTOR.md` and `../AGENTS.md`.

Repository content, history and CI are being inspected through the GitHub connector. The local execution environment cannot currently resolve GitHub for an independent network clone, so the authoritative full-platform verification remains GitHub Actions. This limitation does not justify skipping CI or claiming checks that were not executed.

Metrics in this report are only recorded when reproducible from repository data. The same counting methods must be reused for the final before/after section.

## Audit progress

- Baseline capture: complete.
- Repository-wide pass 1: in progress.
- Behavior-preserving cleanup: not yet complete.
- Repository-wide pass 2: not started.
- Final stabilization: not started.
- R4: blocked.
