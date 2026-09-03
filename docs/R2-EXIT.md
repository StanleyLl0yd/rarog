# R2 — Flight exit audit

Status: **complete** once this document's merge commit passes the normal post-merge `main` CI.

R2 exists to establish Rarog's first script-facing Web-platform, scheduling, network/security-identity and Windows host-event foundations while preserving Rarog-owned boundaries around third-party runtimes and native platform APIs. Exit is based on the scoped work in `R2-BACKLOG.md`, not on general-Web completeness.

## What R2 proves

### WebIDL and script runtime boundaries

R2 owns a normalized WebIDL IR, standards-oriented parser adapter and deterministic binding metadata/validation layer without leaking parser AST types downstream. The replaceable `rarog-script` API owns realms, globals, rooted-value identities, diagnostics, exceptions and resource limits independently of any concrete JavaScript engine.

The first production backend is isolated in `rarog-script-spidermonkey`, pinned to the SpiderMonkey ESR 140 line. Persistent globals and values remain behind opaque Rarog handles, JavaScript throws remain completion semantics rather than backend failures, realm teardown invalidates roots before native document storage can disappear, and the required JSAPI unsafe boundary is confined to the backend crate. Dedicated locked Linux and Windows CI jobs exercise the real backend.

### Events, scheduling and rendering checkpoints

R2 adds engine-owned Event/EventTarget state, task and microtask scheduling, explicit work identities and bounded queues. Script-facing callback execution stays outside registry and scheduler borrows.

The engine bridge performs retained rendering when a microtask checkpoint drains. DOM mutations performed while task or microtask work is executing therefore flow through the same mutation tracking, invalidation, layout and retained-paint machinery introduced by R0/R1 instead of a second script-only rendering path.

### URL, origin and Fetch foundations

R2 adds Rarog-owned URL values behind a standards-oriented URL parser, tuple and opaque origin identities, schemeful-site calculation using a pinned public-suffix snapshot, and bounded Fetch request/response foundations. The network capability receives a deliberately narrower transport request so origin, credentials, redirect and related Web security policy remain owned by Rarog rather than delegated to an embedder transport backend.

### Windows input, IME and clipboard adapters

R2 extends the platform-neutral host boundary with keyboard, pointer, wheel, text-composition and clipboard contracts. The Windows adapter normalizes physical/named keyboard state and mouse input without leaking Win32 message representations; printable text remains layout-aware through character messages rather than virtual-key guesses.

Windows text input retains validated caret state and bridges UTF-16 composition/result payloads into ordered Rarog text events while keeping HWND/HIMC ownership outside the neutral API. The first clipboard adapter supports bounded plain text through Windows Unicode clipboard semantics while native locks, handles and UTF-16 storage remain inside the Windows implementation boundary.

## Explicitly not required for R2 exit

The following work is intentionally deferred:

- broad generated DOM/Web API bindings, large Web API surface coverage, timers/promises integration beyond the established runtime and scheduling foundations — later measured Web-platform work;
- flexbox/grid, compositor thread, `wgpu`, Windows GPU integration, asynchronous image decode, scroll tree and frame scheduler — R3;
- Host/Site processes, IPC, capability broker, Windows sandbox hardening, site isolation and crash recovery — R4;
- storage, workers/service workers, WebSocket, media, canvas/WebGL and accessibility — R5;
- broad WPT/real-Web compatibility qualification and automation protocols — R6;
- stable embedding ABI and additional platform bindings — R7;
- reference browser UI — R8.

R2 therefore must not be described as standards-complete, generally Web-compatible, safe for arbitrary hostile Web content, GPU accelerated or browser-ready.

## Automated exit gate

`crates/rarog-engine/tests/r2_exit.rs` is the Flight milestone gate. It verifies that `R2-BACKLOG.md` is marked complete with no unchecked milestone items, exercises the central task/microtask-to-render-checkpoint bridge with real DOM mutations and retained rendering, and verifies that generic input, IME and clipboard capabilities remain independently representable in the platform contract.

Windows-primary and Linux-portability CI run this gate explicitly in addition to the complete workspace tests, R0/P1/R0.1/R1 gates, fuzz-target compilation, bootstrap render and Rust 1.85 MSRV check. Dedicated SpiderMonkey Linux and Windows jobs continue to validate the concrete JavaScript backend separately.

The R2 backlog becomes historical scope documentation after exit. New functionality belongs to the next appropriate roadmap milestone unless an actual Flight invariant is found to be incorrect.

## Release identity

The workspace remains version `0.1.0`. After the exit PR and its post-merge `main` CI are green, that merge commit is the canonical source point for the `r2-flight` milestone tag.
