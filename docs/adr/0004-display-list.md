# ADR-0004: Layout never paints directly

**Status:** Accepted

## Decision

Layout produces geometry/fragments. A separate paint stage produces a backend-neutral display list. Rasterization/composition consumes the display list.

## Reason

This enables retained rendering, damage tracking, GPU backends, headless tests and process/thread separation without coupling layout to a graphics API.
