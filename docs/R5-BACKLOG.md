# R5 — Web backlog

Status: **in progress**.

Tracking issue: #260.

## A — Storage process

- [x] Add a Rarog-owned Storage-process identity independent of OS PID/handles (#261).
- [x] Add bounded exact-origin storage-process state with explicit key/value, entry, origin and byte quotas (#261).
- [x] Bind production navigation contexts to exact origin authority and route Storage access through Host authorization (#263).
- [x] Define storage IPC/request contracts without exposing filesystem paths or backend handles to Site code (#265).
- [x] Add Windows-first Storage-process launch/loss/containment evidence (#267).
- [x] Add durability/transaction foundations required by the selected Web Storage/IndexedDB slices without claiming broader API completeness.

## B — Workers / Service Workers

- [x] Add bounded worker identities, ownership and lifecycle.
- [x] Add dedicated worker task/microtask execution ownership behind the Rarog Script API.
- [x] Add Worker message delivery with bounded structured payload ownership.
- [ ] Add Service Worker registration/scope/lifecycle foundation.
- [ ] Keep Service Worker fetch interception behind Rarog Fetch/Host security policy.

## C — WebSocket

- [ ] Define Rarog-owned WebSocket URL/handshake/state/message contracts.
- [ ] Keep socket/backend identities private behind Host/network capability routing.
- [ ] Bound inbound/outbound message size and queued bytes/messages.
- [ ] Add close/error/backpressure lifecycle coverage.

## D — Audio / video

- [ ] Define platform-neutral bounded media resource/stream/playback ownership.
- [ ] Keep demux/codec/device/platform APIs behind replaceable adapters.
- [ ] Integrate media scheduling with lifecycle/background resource policy.
- [ ] Add Windows-first media backend foundation without leaking Windows types into Web semantics.

## E — Canvas / WebGL

- [ ] Add bounded Canvas 2D surface/context/state ownership.
- [ ] Connect Canvas output to resource revision/paint/compositor invalidation.
- [ ] Add a WebGL context/resource abstraction with explicit GPU limits and loss semantics.
- [ ] Keep graphics backend objects outside DOM/Web API contracts.

## F — Accessibility

- [ ] Add a platform-neutral accessibility tree with stable derived identities, roles, names, states and bounds.
- [ ] Define actions/events without making platform accessibility objects authoritative Web state.
- [ ] Add deterministic DOM/render-to-accessibility invalidation.
- [ ] Add the Windows accessibility bridge first behind `rarog-platform`/`rarog-platform-windows`.

## G — Milestone exit

- [ ] Complete every R5 backlog item selected above.
- [ ] Add `crates/rarog-engine/tests/r5_exit.rs`.
- [ ] Run the R5 exit gate explicitly in Windows-primary and Linux-portability CI.
- [ ] Preserve R0–R4 exit gates, MSRV 1.85, SpiderMonkey and repository security checks.
- [ ] Reconcile architecture/ADR/README state with implemented evidence.
- [ ] Confirm merged `main` CI before closing #260.

## Stop boundary

R5 completion does not begin or claim R6 compatibility qualification. WPT dashboards, real-Web corpus qualification, signed compatibility profiles, WebDriver/BiDi qualification and Windows real-machine compatibility runs remain R6.
