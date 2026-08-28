# ADR-0029: Engine and embedder boundary

## Status

Accepted.

## Context

R0 already has a deterministic renderer and a stateful `RenderSession`, but callers still enter through renderer-oriented helper functions. Before R1 standards work begins, Rarog needs a stable host-facing ownership boundary that does not assume a browser UI, network stack, or platform process model lives in the engine crate.

## Decision

`rarog-engine` exposes `Engine`, `EngineBuilder`, `View`, and `ViewOptions` as the R0 embedder boundary. `Engine` owns shared host policy, event delivery, resource budgets, and view-ID allocation. A `View` owns one loaded inline document and its active `RenderSession`.

Navigation and resource loading are represented by `NavigationRequest` and `ResourceRequest`. R0 does not perform network I/O. Requests are checked by `HostPolicy` and either blocked or returned as `RequestDisposition::ForwardToEmbedder`; matching `ViewEvent` values are delivered through an UI-neutral `EventSink`.

`ResourceBudget` starts with two enforced limits that already have deterministic ownership boundaries: decoded document-source bytes and framebuffer viewport pixels. The viewport budget may not exceed the lower-level framebuffer safety limit. Memory/cache/background CPU/lifecycle budgets remain future extensions rather than unmeasured R0 estimates.

`View::load_html` accepts already-decoded text plus an opaque `BaseUrl`. `BaseUrl` is deliberately not a standards URL parser. `View::render` creates a `RenderSession` on first use, reuses it for the same viewport, and rebuilds it when the viewport changes.

## Consequences

- Browser/UI code can embed Rarog through `Engine`/`View` instead of renderer internals.
- Host policy and event delivery do not depend on Win32, networking, or a widget toolkit.
- Navigation/resource requests have a stable handoff contract before a network service exists.
- Resource limits are explicit at the embedder boundary and enforced before document parsing or framebuffer allocation where possible.
- `RenderSession` remains the R0 stateful rendering implementation behind `View`; the embedder API can survive later renderer/process refactors.
- This ADR does not define URL parsing, HTTP, redirects, history, origin policy, browser UI, process isolation, or a production resource scheduler.
