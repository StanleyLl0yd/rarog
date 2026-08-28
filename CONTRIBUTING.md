# Contributing to Rarog

Rarog is architecture-first and measurement-driven.

The primary product target is **Windows 10/11**. Engine-core changes must still preserve platform boundaries so later Linux/macOS ports do not require rewriting Web semantics.

Use short-lived topic branches and merge through pull requests. Do not use `main` as a working branch. A PR should be merged only after the required CI jobs pass, and its topic branch should be deleted after merge.

Before adding a subsystem:

1. identify its trust boundary;
2. define ownership/lifetime independently of process placement;
3. define measurable correctness/performance criteria;
4. avoid site-specific behavior in standards code;
5. prefer a narrow adapter over leaking a third-party or OS API across crates;
6. keep Windows-specific APIs behind platform adapters;
7. keep mutation → invalidation → layout → paint dependencies explicit rather than using hidden cross-crate side effects;
8. preserve deterministic output for committed R0 fixtures unless the intentional change is reviewed and the regression fingerprint is updated;
9. treat incremental rendering as an optimization with a deterministic full-rebuild fallback, never as a reason to weaken correctness boundaries.

Changes that knowingly reduce site isolation, origin isolation or capability boundaries for performance are not accepted as normal optimizations.

When changing invalidation or incremental rendering, add tests for both the reuse path and the conservative fallback. A paint-only mutation should prove which derived state was reused; a geometry/structure mutation should prove that the fallback still produces correct deterministic output. The dedicated `r01_correctness` integration target is the required high-level regression gate; unit tests remain the place for narrow subsystem invariants.

Before opening a PR, run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rarog-engine --test r0_exit
cargo test -p rarog-engine --test p1_exit
cargo test -p rarog-engine --test r01_correctness
cargo run -p rarog-shell -- examples/hello.html rarog.ppm
```

If a change intentionally alters the deterministic R0 render signature, explain which DOM/style/layout/fragment/display-list behavior changed and why. Do not update a golden hash only to silence CI without understanding the pipeline difference.

GitHub Actions runs the full quality path on Windows and a portability path on Linux. Push CI runs only for `main`; pull requests run the same gates before merge.

Post-R0 hardening and R1 preflight work is tracked in `docs/R0.1-BACKLOG.md` and issue #32. Do not reopen the historical R0 backlog for later milestone work unless an actual Ember invariant is found to be incorrect.
