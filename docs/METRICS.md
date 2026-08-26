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

As the R0/R1 invalidation path becomes incremental, measure:

- nodes marked style/layout/paint dirty per mutation;
- style rules reconsidered per mutation;
- layout nodes/fragments rebuilt per frame;
- display items changed per frame;
- damaged pixel area versus viewport area;
- unnecessary full-document/full-viewport rebuild count.

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
