# R5 — Web exit gate

Status: **in progress**.

Tracking issue: #260.

## Purpose

R5 expands the R4-isolated engine with bounded storage, workers/service workers, WebSocket, media, Canvas/WebGL and accessibility foundations while preserving Rarog-owned Web semantics and Host-controlled OS authority.

The milestone is intentionally sliced. Each subsystem must establish ownership, lifetime, trust and resource limits before broad API coverage. R5 exit is an architecture/correctness gate, not a claim of general-Web compatibility.

## Required automated gate

Before R5 can be complete, `crates/rarog-engine/tests/r5_exit.rs` must verify representative public contracts across all completed R5 workstreams, while narrower crate/platform tests retain detailed fault-injection and backend evidence.

The final gate must prove at minimum:

- Storage-process identity is independent from Site identities and is fresh after retirement/recovery.
- Persistent storage is keyed by exact Rarog origin identity; same-site cross-origin contexts cannot share state by site identity alone.
- Storage/worker/WebSocket/media/Canvas/WebGL/accessibility externally influenced state is bounded by explicit limits.
- Web-controlled code cannot select filesystem paths, OS handles, socket/backend tickets, codec/device handles, GPU backend objects or platform accessibility objects as authority.
- Worker/service-worker lifecycle and message queues fail closed under stale identity, shutdown and backpressure.
- WebSocket backend lifetime and queued payload accounting are bounded and stale authority cannot reach the backend.
- Media and Canvas/WebGL resource loss/recovery do not turn derived/backend state into Web semantic authority.
- Accessibility state is derived from engine-owned Web/DOM/render state and the Windows bridge remains an adapter rather than a source of semantics.
- Existing R0–R4 exit gates remain green.

## Platform evidence

Windows remains primary. R5 completion requires Windows evidence for the Storage-process lifecycle/containment slice, media/platform integration selected for R5, and the accessibility bridge. Portable contracts and the R5 exit gate must continue to run on Linux.

## Security invariants

- Exact origin, not schemeful site alone, keys persistent storage authority.
- Host/Web content remain distinct trust domains.
- R4 process/navigation-context/capability authority is not weakened.
- All new queues, tables, payloads and resource ownership are explicitly bounded.
- Windows/native APIs remain behind reviewed platform boundaries.
- Unsafe Rust remains confined to existing explicitly reviewed native/runtime boundaries unless a separate architectural decision establishes a narrower required boundary.

## Explicit deferrals / R6 stop

R5 does not include compatibility qualification. WPT dashboards, real-Web corpus qualification, signed compatibility profiles, high-priority Web-app qualification, WebDriver/BiDi and Windows real-machine compatibility runs are R6 and must not begin before R5 is complete and explicitly exited.
