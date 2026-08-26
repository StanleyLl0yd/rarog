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

## Incremental rendering

R0 now has a first stateful incremental experiment. Each `RenderSession` update reports whether it used:

- `Unchanged` — no render-relevant dirty state remained;
- `PaintOnlyReuse` — computed paint values changed while existing Layout Tree / Fragment Tree geometry was reused;
- `FullRebuild` — structure, text or geometry required a conservative rebuild.

Track:

- nodes marked style/layout/paint dirty per mutation;
- dirty nodes accumulated between renders;
- nodes whose computed style was patched without relayout;
- incremental mode counts and full-rebuild fallback rate;
- style rules reconsidered per mutation;
- layout nodes/fragments rebuilt per frame;
- display items changed per frame;
- damaged pixel area versus viewport area;
- unnecessary full-document/full-viewport rebuild count;
- time spent in dirty capture, style comparison, relayout, display-list generation and raster separately.

The current paint-only experiment still rebuilds the display list and rerasterizes the framebuffer. It therefore proves state reuse and correctness boundaries, not a performance win. Retained display-list updates, damage-scoped rasterization and geometry-affecting incremental relayout must be measured separately when implemented.

These numbers are diagnostic until the incremental architecture is mature enough for product targets.

## Safety/reliability

- renderer/site-process crashes
- host-process crashes
- sandbox escapes/security reports
- OOM recoveries
- discarded-page restoration failures

## Comparative benchmarks

Comparisons with Chromium/Firefox/WebKit must use:

- identical hardware/OS;
- equivalent cold/warm state;
- same page corpus;
- documented versions;
- multiple runs and distribution, not a single best number.

The default comparison environment for Windows-facing Rarog/Zorya claims is Windows. Cross-OS comparisons must be labeled as such rather than presented as direct product comparisons.
