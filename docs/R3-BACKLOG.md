# R3 — Wings backlog

Status: **in progress**.

Tracking issue: #109.

## A — Flexbox and Grid

- [x] Introduce the Rarog-owned bounded single-line flex-row geometry/placement primitive (#110).
- [x] Connect `display: flex` computed style and layout-tree dispatch to the measured row algorithm (#136).
- [x] Add flex grow/shrink and main-axis free-space distribution (#137).
- [ ] Add flex alignment, wrapping and reverse directions in measured slices (main-axis `justify-content`: #138; single-row `gap`: #139; container cross-axis `align-items`: #140; per-item `align-self`: #141; definite-row auto-height stretch: #142; bounded multi-line `flex-wrap`: #143; wrapped-line `align-content`: #144).
- [ ] Introduce Rarog-owned grid track/item metadata and a first measured grid layout slice.
- [ ] Expand grid sizing and placement incrementally.

## B — Compositor and GPU

- [ ] Define compositor/frame graph contracts independent of graphics backends.
- [ ] Add a replaceable `wgpu` graphics backend.
- [ ] Add Windows-first GPU surface/device integration.
- [ ] Connect retained display-list damage to compositor updates.

## C — Async resources and scrolling

- [ ] Add an asynchronous image-decode boundary and completion scheduling.
- [ ] Add a scroll tree with stable engine-owned identities.
- [ ] Add engine-owned frame scheduling and presentation boundaries.
- [ ] Connect scroll/resource completion to damage and frame production.

## Scope boundary

R3 does not introduce Host/Site processes, IPC, sandboxing or site isolation; those remain R4. Broad storage, workers, media and accessibility remain R5. Compatibility qualification, stable embedding ABI and browser UI remain later milestones.
