# ADR-0005: No hidden Chromium fallback

**Status:** Accepted

## Decision

Rarog may implement compatibility quirks but must not silently invoke Blink/Chromium for unsupported pages.

## Reason

A hidden fallback would invalidate resource measurements, weaken independence and mask compatibility gaps instead of fixing them.
