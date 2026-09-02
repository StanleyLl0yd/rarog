# Repository Agent Rules

These rules apply to all automated coding agents and repository-wide maintenance work in Rarog.

## Project identity and priorities

Rarog is an independent, Rust-first Web engine.

Preserve these priorities, in order:

1. Web compatibility and standards correctness.
2. Security and trust-boundary preservation.
3. Deterministic correctness.
4. Resource efficiency.
5. Embeddability and platform portability.

Rarog is Windows-first, not Windows-only. Windows 10/11 is the primary implementation target, but platform priority must not leak Windows-specific semantics into the portable engine core.

Zorya Browser is the reference host, not the only supported architectural host. Keep embedding a first-class boundary.

Do not encode temporary milestone status into durable architecture rules. For current scope and milestone state, use `docs/ROADMAP.md` and the applicable exit, backlog, and ADR documents.

Do not claim standards completeness, production security, browser readiness, or performance leadership beyond what the repository's current evidence supports.

## Read before changing architecture

Before changing an engine subsystem, inspect the relevant parts of:

- `docs/ARCHITECTURE.md`;
- `docs/ROADMAP.md`;
- applicable ADRs under `docs/adr/`;
- applicable milestone exit or backlog documents;
- `CONTRIBUTING.md`.

Architecture documentation is part of the contract. When a durable architectural decision changes, update the relevant documentation or ADR in the same work.

## Authoritative architecture

Preserve the semantic flow:

```text
Web input
  -> DOM and mutation records
  -> style / invalidation
  -> layout
  -> fragments
  -> display list / paint
  -> raster / compositor
  -> platform backend
```

Keep dependencies and ownership explicit. Do not introduce reverse ownership, hidden cross-layer mutation, or parallel sources of Web semantics.

DOM and other explicit Web input/state are source state. Layout nodes, fragments, display lists, damage state, framebuffers, caches, and other rendering products are derived state.

Derived state must never become an independent source of DOM, style, layout, or compatibility semantics.

`NodeId`, `LayoutNodeId`, and `FragmentId` are different identity domains. Numeric equality across those domains has no semantic meaning.

Layout must not paint directly. Paint must consume derived layout/fragment output rather than reconstructing Web semantics.

DOM mutation code owns DOM tree invariants. Callers must not repair or bypass DOM ownership rules manually after a mutation.

Mutation -> invalidation -> layout -> paint dependencies must remain explicit rather than being replaced by hidden side effects between crates.

## Platform and trust boundaries

Keep Windows-specific APIs behind the platform abstraction boundary.

Win32, WinRT, Direct3D, Windows accessibility, Windows input, Windows sandboxing, and other OS-specific APIs must not leak into DOM, HTML, CSS, layout, platform-neutral paint semantics, or other portable engine crates.

Use `rarog-platform` for platform-neutral capability contracts and `rarog-platform-windows` for Windows-specific implementations unless an explicitly reviewed architectural change establishes a better boundary.

Host code and Web content are separate trust domains. Web-controlled state must not directly own OS capabilities.

Do not weaken site isolation, origin isolation, capability boundaries, or future process boundaries as a normal performance optimization.

Crate boundaries that model future security or process boundaries must remain meaningful even while the bootstrap implementation runs in fewer processes.

## Rust and memory safety

Preserve the workspace MSRV unless a concrete toolchain or dependency requirement justifies changing it.

Preserve the workspace-level `unsafe_code = "forbid"` policy. Do not weaken it as part of ordinary implementation, dependency integration, optimization, or refactoring work.

If a future platform integration genuinely requires `unsafe`, isolate it behind the narrowest reviewed platform boundary and change the workspace safety policy only through an explicit architectural decision.

Prefer ownership and lifetime models that make invalid states difficult to represent rather than relying on defensive runtime repair.

Do not hide correctness failures with broad error suppression or unchecked conversions.

## Untrusted input and bounded resources

Treat Web-controlled HTML, CSS, URLs, dimensions, text, future script-visible data, and other external input as untrusted.

Malformed or hostile input must not cause uncontrolled panics, unbounded allocation, unbounded recursion, unbounded journals, or unbounded cache growth at public engine boundaries.

Keep source buffers, decoded input, mutation history, caches, parser state, layout work, display lists, damage tracking, raster buffers, framebuffers, and similar structures bounded by explicit ownership, lifetime, or resource budgets.

Do not introduce process-global immortal caches, registries, or intern tables unless their isolation, lifetime, invalidation, and memory bounds are explicitly justified.

Prefer fallible public boundaries when input or allocation failure is possible.

## Compatibility and semantic fallbacks

Compatibility is a correctness requirement, not permission to guess.

Do not silently fabricate plausible Web semantics for an unsupported or ambiguous standards feature merely to make a site appear to work.

Site-specific compatibility behavior belongs in a separate, auditable compatibility boundary rather than in standards implementation code.

A conservative internal fallback is appropriate when it preserves known-correct semantics, such as rebuilding derived render state instead of reusing state whose validity cannot be proven.

A conservative fallback must not silently convert unsupported external semantics into a knowingly incorrect result.

When a compatibility, parser, invalidation, layout, paint, or rendering bug is fixed, add the smallest practical deterministic regression case that proves the corrected behavior.

## Incremental rendering and invalidation

Incremental rendering is an optimization, never the correctness authority.

Reuse derived state only when the engine can demonstrate that the retained state remains valid for the relevant mutation and dependency scope.

When reuse cannot be proven safe, use the deterministic conservative rebuild path.

Changes to invalidation or incremental rendering must test both:

- the intended reuse path;
- the conservative fallback path.

Do not weaken the full-rebuild fallback merely to increase reuse rates.

Do not update deterministic snapshots, signatures, or golden hashes merely to silence a failing test. First identify and explain the semantic pipeline change that caused the difference.

## Determinism and performance

Equivalent input on the same supported architecture and toolchain should produce equivalent committed deterministic snapshots, display identities, and framebuffer signatures unless an intentional semantic change requires updating them.

Timing and benchmark data are diagnostics. They must not feed deterministic render identity.

Performance work must be measurement-driven and should target demonstrated cost rather than intuition alone.

Do not trade correctness, security boundaries, portability, or architectural clarity for an unmeasured optimization.

Do not use fragile cross-machine wall-clock thresholds as correctness or compatibility gates.

Keep performance-sensitive work bounded and ensure optimization-specific caches have correct semantic keys and lifetimes.

## Dependencies and generated code

Add a dependency only for a concrete current need.

Prefer narrow adapters around replaceable engines and platform services. Networking, graphics, JavaScript-engine, OS, and other third-party APIs must not leak across unrelated crates.

Do not replace a mature dependency with custom code solely to reduce dependency count. Do not add overlapping libraries when the existing stack already provides the required capability adequately.

Keep dependency versions, lockfiles, and CI configuration reproducible and synchronized when dependency metadata changes.

Keep third-party GitHub Actions pinned to immutable full commit SHAs.

Do not weaken existing CI, portability, MSRV, fuzz-build, or correctness gates to make a change pass.

Do not edit generated artifacts as though they were authoritative source when a schema, generator, or other upstream source of truth exists. Change the source of truth and regenerate instead.

## Verification

Run the checks appropriate to the change before considering it complete.

The baseline repository verification is:

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

Changes affecting portability or platform boundaries must preserve both Windows-primary and Linux-portability CI behavior.

Changes affecting toolchain compatibility must preserve the declared MSRV check.

Changes affecting parsers, mutation logic, externally controlled input, or other fuzz-relevant boundaries must keep fuzz targets building and should run the relevant bounded fuzzing when the environment supports it.

Never claim a check passed unless it actually ran successfully. State unavailable tooling, platforms, credentials, hardware, or other verification limitations explicitly.

## Change discipline

Use short-lived topic branches and pull requests. Do not use `main` as a working branch.

Keep commits and pull requests focused on one coherent purpose. Separate behavior-preserving refactoring from unrelated feature development.

Do not force-push shared history, discard unrelated user changes, or weaken branch protection without explicit authorization.

A pull request should merge only after the required checks pass. Remove obsolete topic branches after merge or intentional abandonment when practical.

Never commit credentials, tokens, private keys, signing material, generated secrets, local environment data, or sensitive test data.

## Comments and documentation

Keep source-code comments minimal, necessary, current, and English-only.

Do not add comments that merely narrate obvious code. Prefer self-explanatory names and structure.

Keep comments that explain non-obvious invariants, ownership, safety constraints, compatibility behavior, resource bounds, or architectural reasons.

Remove stale, misleading, redundant, and commented-out historical code when the surrounding change proves it is obsolete.

When behavior, architecture, supported commands, toolchain requirements, or durable subsystem contracts change, review and update the relevant repository documentation in the same work.

## Repository-wide audit and deep refactoring

For a full repository audit, cleanup, optimization, simplification, or deep-refactoring task, read and follow `docs/agent/AUDIT_REFACTOR.md` in full before editing.

The Rarog-specific invariants in this file remain mandatory throughout that process and take precedence over generic simplification goals.
