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
- [ ] Continue Grid sizing/placement with intrinsic and content-driven track sizing without coupling CSS parser AST types to layout.

## B — Compositor and GPU

- [x] Define compositor/frame graph contracts independent of graphics backends (#152).
- [x] Add a replaceable `wgpu` graphics backend (#154; staged retained raster upload).
- [x] Add Windows-first GPU device, safe surface lifecycle and retained presentation integration (#155–#157, #159, #163).
- [x] Connect retained display-list revision/damage lifecycle to compositor updates (#153).
- [x] Add an owned frame packet suitable for crossing execution/lifetime boundaries (#176).
- [ ] Move compositor execution onto a bounded worker thread (#178 in progress), then move the Windows presenting backend onto that worker without leaking platform/GPU types into engine contracts.

## C — Async resources and scrolling

- [x] Add a bounded asynchronous image-decode boundary and connect completion to retained image revisions/frame scheduling (#161, #165, #167–#171).
- [x] Add a bounded scroll tree with stable engine-owned identities (#162).
- [x] Add engine-owned frame scheduling and Windows presentation boundaries (#158–#160, #164, #169).
- [x] Connect root scrolling and resource completion to retained damage/frame production (#171, #172–#175).

## Current focus

The retained Windows frame path now reaches real DX12/wgpu surface presentation, image decode completion participates in retained frame production, and the root scroll node drives actual viewport translation/damage without changing display-list identity. The next compositor slice is the bounded worker execution boundary; broader CSS overflow/nested scroll-container semantics and intrinsic Grid sizing remain separate standards work.

## Scope boundary

R3 does not introduce Host/Site processes, IPC, sandboxing or site isolation; those remain R4. Broad storage, workers, media and accessibility remain R5. Compatibility qualification, stable embedding ABI and browser UI remain later milestones.
