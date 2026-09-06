# R3 — Wings backlog

Status: **in progress**.

Tracking issue: #109.

## A — Flexbox and Grid

- [x] Introduce the Rarog-owned bounded single-line flex-row geometry/placement primitive (#110).
- [x] Connect `display: flex` computed style and layout-tree dispatch to the measured row algorithm (#136).
- [x] Add flex grow/shrink and main-axis free-space distribution (#137).
- [x] Add flex alignment, wrapping and reverse directions in measured slices (main-axis `justify-content`: #138; single-row `gap`: #139; container cross-axis `align-items`: #140; per-item `align-self`: #141; definite-row auto-height stretch: #142; bounded multi-line `flex-wrap`: #143; wrapped-line `align-content`: #144; measured wrapped auto-height stretch: #145; cross-axis `wrap-reverse`: #146; main-axis `row-reverse`: #147).
- [x] Introduce Rarog-owned grid track/item metadata and a first measured grid layout slice (#148).
- [x] Expand the first bounded Grid slices through fixed CSS Grid, explicit-grid auto-placement and item self-alignment (#149–#151).
- [x] Add the first bounded intrinsic/content-driven Grid track-sizing slice with single-span `auto` tracks (#182).
- [x] Add intrinsic automatic sizing for non-stretch Grid items (#183).
- [x] Add bounded Grid container default inline alignment through `justify-items` (#184).
- [x] Introduce explicit layout-owned Grid track sizing state with base sizes and growth limits (#185).
- [x] Add an order-independent, gap-aware spanning base-size distribution primitive without wiring incomplete contribution semantics into CSS layout (#186).
- [x] Add explicit layout-owned Grid intrinsic contribution classes for minimum/min-content/max-content values (#187).
- [x] Add span-ordered intrinsic sizing rounds over synthetic Grid contributions (#188).
- [x] Derive bounded semantic Grid minimum/min-content/max-content contributions from retained layout measurements without changing current geometry (#189).
- [ ] Replace the max-content compatibility selection with bounded intrinsic base-size/growth-limit phases before enabling spanning intrinsic layout.
- [ ] Continue Grid fractional/max-track sizing without coupling CSS parser AST types to layout.

## B — Compositor and GPU

- [x] Define compositor/frame graph contracts independent of graphics backends (#152).
- [x] Add a replaceable `wgpu` graphics backend (#154; staged retained raster upload).
- [x] Add Windows-first GPU device, safe surface lifecycle and retained presentation integration (#155–#157, #159, #163).
- [x] Connect retained display-list revision/damage lifecycle to compositor updates (#153).
- [x] Add an owned frame packet suitable for crossing execution/lifetime boundaries (#176).
- [x] Move backend-neutral compositor execution onto a bounded worker thread with one-frame backpressure (#178).
- [x] Move the Windows presenting backend onto the compositor worker without leaking platform/GPU types into engine contracts (#181).

## C — Async resources and scrolling

- [x] Add a bounded asynchronous image-decode boundary and connect completion to retained image revisions/frame scheduling (#161, #165, #167–#171).
- [x] Add a bounded scroll tree with stable engine-owned identities (#162).
- [x] Add engine-owned frame scheduling and Windows presentation boundaries (#158–#160, #164, #169).
- [x] Connect root scrolling and resource completion to retained damage/frame production (#171, #172–#175).

## Current focus

The retained Windows frame path now reaches real DX12/wgpu presentation through the bounded compositor worker: the worker owns the Windows GPU device, surface, retained wgpu backend and surface recovery while engine/frame contracts remain backend-neutral. Image decode completion participates in retained frame production, and the root scroll node drives actual viewport translation/damage without changing display-list identity. Grid now supports bounded single-span `auto` track sizing (#182), intrinsic automatic sizing for non-stretch items (#183), container default inline alignment through `justify-items` (#184), and an explicit internal base-size/growth-limit sizing state (#185). The first order-independent, gap-aware spanning distribution primitive now exists on that state (#186) but is intentionally not wired into CSS layout yet. Minimum/min-content/max-content contribution classes are now explicit layout-owned data (#187). Span-ordered sizing rounds now compose synthetic distribution state in increasing-span phases (#188). Bounded semantic contribution derivation now preserves inline minimum/min-content/max-content measurements through the fragment builder (#189) while production geometry deliberately retains the #182 max-content compatibility selection. The next Grid work must introduce separate intrinsic base-size and growth-limit phases before enabling spanning intrinsic layout; broader CSS overflow and nested scroll-container semantics remain separate work.

## Scope boundary

R3 does not introduce Host/Site processes, IPC, sandboxing or site isolation; those remain R4. Broad storage, workers, media and accessibility remain R5. Compatibility qualification, stable embedding ABI and browser UI remain later milestones.
