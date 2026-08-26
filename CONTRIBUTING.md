# Contributing to Rarog

Rarog is architecture-first and measurement-driven.

The primary product target is **Windows 10/11**. Engine-core changes must still preserve platform boundaries so later Linux/macOS ports do not require rewriting Web semantics.

Before adding a subsystem:

1. identify its trust boundary;
2. define ownership/lifetime independently of process placement;
3. define measurable correctness/performance criteria;
4. avoid site-specific behavior in standards code;
5. prefer a narrow adapter over leaking a third-party or OS API across crates;
6. keep Windows-specific APIs behind platform adapters;
7. keep mutation → invalidation → layout → paint dependencies explicit rather than using hidden cross-crate side effects;
8. preserve deterministic output for committed R0 fixtures unless the intentional change is reviewed and the regression fingerprint is updated.

Changes that knowingly reduce site isolation, origin isolation or capability boundaries for performance are not accepted as normal optimizations.

Before opening a PR, run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rarog-engine deterministic_render_snapshot_and_hash
cargo run -p rarog-shell -- examples/hello.html rarog.ppm
```

If a change intentionally alters the deterministic R0 render signature, explain which DOM/style/layout/fragment/display-list behavior changed and why. Do not update a golden hash only to silence CI without understanding the pipeline difference.

GitHub Actions runs the full quality path on Windows and a portability path on Linux.
