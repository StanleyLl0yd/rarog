# Rarog metrics

Rarog is measurement-driven. Marketing claims must follow measurements, never precede them.

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
- DOM/style/layout retained bytes
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
