# ADR-0058: Fetch request/response and network capability boundary

Status: accepted

## Context

R2 needs Fetch-facing request and response state before navigation, subresource loading and script APIs can share a coherent networking model. The browser engine must own URL/origin policy and Fetch semantics, while an embedder must remain free to choose the actual HTTP/network stack.

Passing a high-level Fetch request directly to a networking library would let transport code accidentally become responsible for CORS, same-origin checks, credentials policy or redirect policy. Conversely, requiring a synchronous transport call would unnecessarily constrain later event-loop and asynchronous integration.

## Decision

Rarog owns Fetch data and lifecycle contracts in the dependency-light `rarog-fetch` crate. The crate depends only on `rarog-url` and does not select an HTTP client, TLS implementation, DNS resolver or platform networking API.

`FetchRequest` owns the high-level request state needed by later Fetch policy: normalized fragment-free URL, request origin, method, ordered header list, optional body, mode, credentials mode, redirect mode, destination and explicit resource limits.

`NetworkRequest` is a transport projection of a `FetchRequest`. It intentionally contains only data required to perform I/O: URL, method, headers, body and the maximum accepted response-body size. Origin, request mode, credentials mode, redirect mode and destination are not exposed through this projection. Later Fetch policy code must decide how those fields affect transport before calling the network capability.

`NetworkCapability` is object-safe and completion-oriented. `start` accepts a `NetworkRequest` and returns a capability-scoped opaque `NetworkTicket`; `poll` returns `Pending` or a completed `FetchResponse`; `cancel` terminates outstanding work. This does not require a particular threading, reactor, async-runtime or callback model and therefore remains usable by future platform/network adapters.

Headers are stored as an ordered list rather than a map so repeated fields remain representable. Header names are validated as HTTP tokens and normalized to lowercase. Header values reject NUL/CR/LF and trim HTTP whitespace at their edges. The initial header model does not yet implement Fetch header guards or forbidden-request/response-header filtering.

Common Fetch methods are normalized, CONNECT/TRACE/TRACK are rejected, and GET/HEAD bodies are rejected. Request URLs and response URLs are fragment-free. Explicit limits bound header count, total header bytes and request/response body sizes before broader streaming support exists.

## Consequences

DOM/script/engine crates can construct policy-rich Fetch requests without depending on a concrete networking stack. Embedders receive a deliberately narrower transport request, which keeps origin and CORS decisions in Rarog-owned code.

The completion-oriented capability can later be driven from `rarog-scheduler` without changing the Fetch data model. A Windows adapter, libcurl/HTTP library adapter, test fixture or remote network service can implement the same boundary.

This slice does not implement CORS processing, redirect following, credentials/cookie storage, cache semantics, referrer policy, service workers, streaming bodies, content decoding, HTTP authentication or mixed-content checks. Those features extend the Rarog-owned Fetch layer rather than moving policy into the network capability.
