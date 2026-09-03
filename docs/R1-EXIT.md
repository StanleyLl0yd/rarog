# R1 — Flame exit audit

Status: **complete** once this document's merge commit passes the normal post-merge `main` CI.

R1 exists to move Rarog from architectural bootstrap semantics toward standards-oriented parsing, formatting, text and retained rendering while preserving Rarog-owned boundaries. Exit is based on the scoped work in `R1-BACKLOG.md`, not on general-Web completeness.

## What R1 proves

### Standards-oriented HTML and CSS foundations

The bootstrap HTML parser has been replaced by the standards-oriented `html5ever` adapter behind Rarog-owned DOM/parser types, including focused tree-builder and WPT-backed coverage. CSS parsing likewise uses a standards-oriented tokenizer/parser adapter, with measured selector expansion, `!important`, inheritance and CSS-wide values.

### Formatting contexts

R1 establishes scoped block and inline formatting-context foundations: adjoining margin collapse, block sizing and BFC boundaries; shared line construction; baseline and `vertical-align`; inline fragmentation with stable ownership/ordinals; and nested/multi-leaf inline streams. This is intentionally a foundation rather than full CSS layout coverage.

### Resource and production text boundaries

R1 adds the decoded-image resource abstraction without yet implementing URL/Fetch or asynchronous image decoding. Production OpenType shaping is connected through the Rarog shaping boundary using HarfRust. Windows has the first system-font discovery/text adapter, and Windows CI exercises system-font resolution into the production shaper without leaking platform-specific types into layout.

### Standards-aware incremental rendering

`RenderSession` no longer treats normal connected DOM/text/style changes as blanket full-layout rebuilds. R1 covers ordinary `CharacterData`, paint-only style updates, structural append/reparent/detach, connected stylesheet-source changes, visibility/display/BFC formatting transitions, complex inline geometry and mixed text/geometry updates through retained layout refresh and flow-aware fragment relayout.

`FullRebuild` remains a fail-closed recovery path for lost mutation history, missing safe retained coverage or retained-refresh invariant failure. Fragment-level retained failures may conservatively fall back to broader geometry relayout without rebuilding the complete Layout Tree.

### Retained paint and damage

Flow-aware fragment relayout can replace the affected display-list suffix instead of rebuilding the entire display list. Partial raster damage replays clip, stacking, transform and opacity scopes, and structural damage is computed from stable display-item identity plus effective transform/clip/opacity state and paint order. Exact retained replacement remains fail-closed: an unprovable range or scope falls back to rebuilding the display list rather than accepting corrupt retained state.

## Explicitly not required for R1 exit

The following work is intentionally deferred:

- WebIDL, script runtime integration, events/event loop and script-driven DOM mutation — R2
- Fetch, URL/origin/security primitives — R2
- Windows input/IME and clipboard host adapters — R2
- flexbox/grid, compositor, `wgpu`, async image decode, scroll tree and frame scheduler — R3
- multi-process site isolation and Windows sandbox/process hardening — R4
- broader Web APIs, media, workers/storage and accessibility — R5
- broad WPT/real-Web compatibility qualification — R6
- stable embedding ABI and additional platform bindings — R7
- reference browser UI — R8

R1 therefore must not be described as standards-complete, generally Web-compatible, safe for arbitrary hostile Web content, GPU accelerated or browser-ready.

## Automated exit gate

`crates/rarog-engine/tests/r1_exit.rs` is the Flame milestone gate. It verifies that `R1-BACKLOG.md` is marked complete with no unchecked milestone items and exercises a representative retained mixed text/geometry update against a fresh render. The update must use `FlowRelayout`, retain the display list and produce the same framebuffer as a full fresh render.

Windows-primary and Linux-portability CI run this gate explicitly in addition to the complete workspace tests, R0/P1/R0.1 gates, fuzz-target compilation, bootstrap render and Rust 1.85 MSRV check.

The R1 backlog becomes historical scope documentation after exit. New functionality belongs to the next appropriate roadmap milestone unless an actual Flame invariant is found to be incorrect.

## Release identity

The workspace remains version `0.1.0`. After the exit PR and its post-merge `main` CI are green, that merge commit is the canonical source point for the `r1-flame` milestone tag.
