# R3 — Wings backlog

Status: **complete**.

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
- [x] Add a non-spanning intrinsic state resolver with independent base-size and growth-limit contribution kinds (#190).
- [x] Add bounded definite-free-space Maximize Tracks and final auto-track stretch phases (#191).
- [x] Preserve `normal` / `stretch` content-distribution semantics in computed style without changing Flex used behavior (#192).
- [x] Wire definite Grid content-box space into the supported start/stretch intrinsic pipeline while keeping auto-height rows indefinite (#193).
- [x] Add Grid track-group positioning/distribution for start/end/center/space-* and remove the remaining content-alignment compatibility path (#194).
- [x] Add composed, gap-aware span-ordered base/growth-limit rounds for the fixed/`auto` intrinsic subset as a test-scoped validation model (#195).
- [x] Promote the validated fixed/`auto` spanning model into CSS-visible Grid geometry for the semantic minimum → max-content path (#196).
- [x] Add layout-owned fractional track metadata plus definite-space `fr` expansion with flex-factor freezing/restart semantics, without exposing CSS syntax yet (#197).
- [x] Wire bounded `fr` syntax into CSS-visible Grid for definite-space, non-flex-spanning geometry (#198).
- [x] Add bounded CSS-visible `min-content` / `max-content` track sizing with track-specific intrinsic contribution selection (#199).

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

## Exit state

R3's bounded Wings scope is complete. Flexbox, Grid, compositor/GPU, asynchronous image decode, scroll-tree and frame-scheduling milestones listed above have explicit implementations and regression coverage.

Grid closes R3 with the deliberately bounded surface established through #182–#199: fixed and `auto` tracks, intrinsic single-span sizing, validated fixed/`auto` intrinsic spanning, content distribution, definite-space `fr` tracks, and bare `min-content` / `max-content` tracks. Unsupported advanced Grid semantics remain fail-closed rather than approximated.

R4 work is **blocked** after R3 exit. The next phase is the mandatory repository-wide audit and deep refactor tracked by `PRE-R4-AUDIT.md` and governed by `agent/AUDIT_REFACTOR.md`. Host/Site processes, IPC, sandboxing and site isolation must not begin until that gate is complete.

## Scope boundary

R3 does not introduce Host/Site processes, IPC, sandboxing or site isolation; those remain R4. Broad storage, workers, media and accessibility remain R5. Compatibility qualification, stable embedding ABI and browser UI remain later milestones.
