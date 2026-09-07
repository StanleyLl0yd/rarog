# R3 — Wings exit gate

Status: **complete** when this document and the automated `r3_exit` gate are merged to `main` with all required CI green.

## Purpose

R3 establishes the bounded rendering/runtime work required before process architecture begins:

- measured Flexbox and CSS Grid slices with explicit fail-closed boundaries;
- backend-neutral compositor/frame contracts;
- bounded compositor worker execution;
- replaceable `wgpu` graphics backend and Windows-first presentation path;
- bounded asynchronous image decode;
- stable engine-owned scroll identities and root scrolling;
- engine-owned frame scheduling.

R3 completion is a milestone boundary, not a standards-completeness claim.

## Automated gate

The integration test `crates/rarog-engine/tests/r3_exit.rs` verifies that:

- the R3 backlog has no unchecked milestone items;
- CSS-visible definite-space `fr` Grid geometry is active;
- CSS-visible `min-content` / `max-content` track geometry is active;
- frame scheduling coalesces causes and preserves requests across discard/retry;
- the bounded image-decode queue transitions a reserved resource from pending to ready and releases retained encoded bytes;
- scroll offsets clamp to content bounds and produce viewport damage.

The complete workspace test suite remains responsible for the deeper compositor-worker, wgpu adapter, retained-frame, Flexbox/Grid and resource regressions accumulated during R3.

CI runs the dedicated R3 gate on both Windows-primary and Linux-portability jobs.

## Architecture invariants preserved

R3 exits with these boundaries intact:

- DOM/CSS/layout/paint APIs do not expose backend-specific GPU or Windows types;
- the graphics backend remains replaceable behind compositor/platform seams;
- Web-controlled resource work is bounded;
- unsupported Grid semantics remain explicit rather than silently approximated;
- Rust 1.85 MSRV, Windows-primary, Linux portability and SpiderMonkey checks remain required.

## Explicit deferrals

R3 does not include:

- Host/Site processes;
- IPC;
- sandboxing;
- capability brokering;
- site isolation;
- crash recovery;
- full CSS Grid grammar or standards completeness;
- storage/workers/media/accessibility;
- browser UI.

Those remain later roadmap work.

## Transition rule

Closing R3 does **not** authorize immediate R4 implementation.

The next phase is the repository-wide audit/refactor and stabilization gate in `PRE-R4-AUDIT.md`. R4 is blocked until that gate is complete.
