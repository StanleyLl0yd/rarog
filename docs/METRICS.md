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
