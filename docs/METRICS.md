# Rarog metrics

Rarog is measurement-driven. Marketing claims must follow measurements, never precede them.

## Measurement platform

Windows is the first release-quality target, so primary product measurements are taken on documented **Windows 10/11** hardware/configurations.

Linux/macOS measurements remain useful for portability and engine analysis, but they must not be silently mixed with Windows numbers in product claims.

## Compatibility

Track separately:

- WPT pass/fail/timeout/crash
- feature-area breakdown (HTML/CSS/DOM/etc.)
- real-Web corpus scenario pass rate
- visual/layout regression count

Focused WPT subsets should begin with the standards-oriented R1 parser/style/layout slices. R6 remains the milestone for the broader WPT dashboard and compatibility program, not the first point at which standards tests are executed.

## Deterministic correctness

R0 tracks deterministic regression fingerprints before performance claims are allowed. For committed fixtures record/compare:

- DOM snapshot changes;
- stylesheet/cascade/computed-style snapshot changes;
- Layout Tree and Fragment Tree snapshot changes;
- display-item ID / display-list changes;
- damage-region changes where a previous frame exists;
- framebuffer stable hash;
- combined deterministic render-signature hash.

A changed fingerprint is evidence to investigate, not automatically a regression: intentional rendering changes may update the expected fingerprint after review.

The high-level deterministic and incremental contract is also exercised by the dedicated `rarog-engine` `r01_correctness` integration target so a required CI gate cannot silently disappear because a filtered unit-test name was removed or renamed.

## Responsiveness

- cold/warm engine initialization
- first contentful render for local fixtures
- input-to-frame latency
- scroll-frame misses at 60/120 Hz

## Resource cost

- resident set by process/site
- JS heap
- DOM/style/layout/fragment retained bytes
- decoded image cache
- graphics cache
- CPU time foreground/background/frozen
- energy where platform instrumentation exists

R0 currently enforces decoded source-byte and framebuffer-pixel budgets at the embedder boundary. R0.1 expands the resource model toward structural limits and safe deep-tree behavior before hostile content is treated as a production security boundary.

## Incremental rendering

`RenderSession` reports one of the following update paths:

- `Unchanged` — no render-relevant dirty state remains;
- `PaintOnlyReuse` — computed paint values change while existing Layout Tree and Fragment Tree geometry is retained; affected display-list ranges are patched when a safe retained range exists;
- `SubtreeRelayout` — footprint-safe geometry is rebuilt for the affected Fragment subtree while the Layout Tree is retained;
- `FlowRelayout` — a vertical-footprint change retains the Layout Tree and unaffected root-flow prefix while rebuilding the earliest affected root-flow child and following siblings;
- `GeometryRelayout` — the Layout Tree is retained but Fragment geometry is rebuilt when a narrower incremental mapping cannot be proven safe;
- `FullRebuild` — structure, text, display membership or another unprovable case uses the deterministic full-rebuild fallback.

Paint retains unaffected display-list ranges when a replacement is structurally valid. The persistent software framebuffer is then updated inside damage rectangles for non-structural display lists. Structural clip/stacking/transform/opacity scopes currently force conservative full-frame raster refreshes where damage-scoped replay is not yet proven safe.

Track:

- nodes marked style/layout/paint dirty per mutation;
- dirty nodes accumulated between renders;
- nodes whose computed style was patched without relayout;
- incremental mode counts and full-rebuild fallback rate;
- style rules reconsidered per mutation;
- layout nodes/fragments rebuilt per frame;
- display items retained, replaced and regenerated per frame;
- damaged pixel area versus viewport area;
- full-frame raster fallbacks caused by structural display scopes;
- unnecessary full-document/full-viewport rebuild count;
- time spent in dirty capture, style comparison, relayout, display-list generation/patching and raster separately.

These numbers remain diagnostic until the incremental architecture and benchmark methodology are mature enough for product targets. Retained state and damage-aware raster correctness do not by themselves establish an end-to-end performance win.

## Safety/reliability

- renderer/site-process crashes
- host-process crashes
- sandbox escapes/security reports
- OOM recoveries
- discarded-page restoration failures
- parser/render no-panic fuzz results
- resource-budget rejections by category

## Comparative benchmarks

Comparisons with Chromium/Firefox/WebKit must use:

- identical hardware/OS;
- equivalent cold/warm state;
- same page corpus;
- documented versions;
- multiple runs and distribution, not a single best number.

The default comparison environment for Windows-facing Rarog/Zorya claims is Windows. Cross-OS comparisons must be labeled as such rather than presented as direct product comparisons.
