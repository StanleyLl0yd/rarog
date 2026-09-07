# R3 — Wings exit audit

Status: **complete** once this document's merge commit passes the normal post-merge `main` CI.

R3 exists to move Rarog from the R2 script/network/input foundations into richer layout and a real retained compositor/frame path while preserving Rarog-owned boundaries around graphics, platform and asynchronous resource implementations. Exit is based on the bounded work in `R3-BACKLOG.md`, not on general CSS Grid or browser completeness.

## What R3 proves

### Flexbox and Grid

R3 establishes bounded Flex row layout with grow/shrink, gaps, alignment, wrapping, reverse directions and measured cross-axis behavior.

Grid owns explicit track/item metadata, placement and self-alignment, intrinsic contribution classes, base-size/growth-limit state, span-ordered fixed/`auto` intrinsic sizing, definite-space maximize/stretch phases and track-group content distribution.

The CSS-visible bounded Grid surface includes fixed and `auto` tracks, validated fixed/`auto` intrinsic spanning, definite-space fractional tracks, and bare `min-content` / `max-content` tracks. Unsupported Grid phases remain explicit failures rather than fabricated compatibility behavior.

### Compositor and GPU

R3 defines backend-neutral frame/compositor contracts, retained display-list revision and damage integration, and a replaceable `wgpu` backend.

The Windows path owns GPU device/surface lifecycle and presentation behind platform/backend boundaries. A bounded compositor worker owns presentation execution with one-frame backpressure while engine-facing frame contracts remain free of Windows and wgpu implementation types.

### Asynchronous resources, scrolling and frame scheduling

R3 adds bounded asynchronous image decode completion into retained image revisions and frame production.

A stable engine-owned scroll tree drives root viewport translation/damage without changing display-list identity.

The frame scheduler owns explicit request identities and coalesced causes so scene changes, scrolling, resource readiness, resizing and explicit requests participate in deterministic frame production.

## Explicitly not required for R3 exit

The following work is intentionally deferred:

- `minmax()`, fit-content, percentages, repeat syntax, named lines, implicit Grid expansion, subgrid, indefinite flex fractions and flexible/content-sized intrinsic spanning;
- broad CSS overflow and nested scroll-container semantics;
- Host/Site processes, IPC, capability broker, Windows sandbox hardening, site isolation and crash recovery — R4, **after the mandatory pre-R4 audit/refactor gate**;
- storage, workers/service workers, WebSocket, media, canvas/WebGL and accessibility — R5;
- broad WPT/real-Web qualification and automation protocols — R6;
- stable embedding ABI and additional platform bindings — R7;
- reference browser UI — R8.

R3 therefore must not be described as CSS-complete, generally Web-compatible, safe for arbitrary hostile Web content, multiprocess, sandboxed or browser-ready.

## Automated exit gate

`crates/rarog-engine/tests/r3_exit.rs` is the Wings milestone gate.

It verifies that `R3-BACKLOG.md` is marked complete with no unchecked milestone items, exercises CSS-visible fractional Grid geometry through the full engine rendering path by comparing it with equivalent fixed-track geometry, and verifies the compositor frame scheduler coalesces independent frame causes while preserving explicit request identity/completion.

Windows-primary and Linux-portability CI run this gate explicitly in addition to the complete workspace tests, prior milestone gates, fuzz-target compilation, bootstrap render and Rust 1.85 MSRV check. Dedicated SpiderMonkey Linux and Windows jobs continue to validate the concrete JavaScript backend separately.

## Mandatory hold before R4

R3 completion does **not** authorize R4 implementation.

The repository must next complete `PRE-R4-AUDIT.md`, following `agent/AUDIT_REFACTOR.md` in full. That phase is behavior-preserving: no unrelated feature work and no Host/Site process, IPC, sandbox or site-isolation rollout.

Only after the pre-R4 audit document is marked complete and its final verification is green may R4 begin.

## Release identity

The workspace remains version `0.1.0`. After the exit PR and its post-merge `main` CI are green, that merge commit is the canonical source point for the `r3-wings` milestone tag.
