# ADR-0081: Windows presentation ownership on the compositor worker

## Status

Accepted.

## Context

R3 already has backend-neutral frame planning, owned frame submissions and a bounded compositor worker. The Windows shell still owned the DX12/wgpu device, surface and retained compositor backend directly, so actual GPU submission and presentation executed on the window/event-loop thread.

Moving that state into engine or paint contracts would violate the platform boundary: window handles, `wgpu` surfaces and recovery rules are Windows adapter concerns, while `FramePlanner` must remain backend-neutral and advance only after confirmed presentation.

## Decision

`rarog-platform-windows` owns a `WindowsPresentingCompositor<T>` adapter containing:

- the surface target retained for surface recreation;
- `WindowsGpuDevice`;
- `WindowsGpuSurface`;
- the retained `WgpuCompositorBackend`.

The adapter implements Rarog's backend-neutral `CompositorBackend` and `PresentingCompositorBackend` contracts.

`rarog-window` owns only the native `Window`, engine `View`, `FramePlanner` and `PresentingCompositorWorker<WindowsGpuError>`. It no longer depends directly on `rarog-compositor-wgpu`.

A submitted frame crosses the worker boundary as an `OwnedFrameSubmission`. Before raster submission, the Windows adapter resizes its owned surface from the backend-neutral `FramePlan::size()`. Presentation then occurs on the named `rarog-compositor` worker.

Presentation recovery remains inside the Windows adapter:

- a suspended surface defers presentation;
- timeout requests a retry;
- outdated surfaces are reconfigured;
- lost surfaces are recreated from the retained target;
- out-of-memory and other fatal surface failures are returned as errors.

Transient recovery maps to `PresentationStatus::Deferred`. The shell requests another redraw and discards the pending planner frame. Only `PresentationStatus::Presented` completes the `FramePlanner` frame and the engine frame request.

Retained-only presentation uses a distinct worker command and never fabricates a `FrameId`. Frame completions are checked against the submitted frame identity before planner state advances.

The current shell waits for each worker completion before returning from the redraw handler. This preserves deterministic one-frame-in-flight semantics while moving GPU ownership and execution off the event-loop thread. Pipelined/non-blocking completion integration is separate scheduling work.

## Consequences

Windows device, surface, recovery and `wgpu` types do not enter DOM, CSS, layout, paint, engine frame-planning or generic compositor contracts.

The Windows surface and retained GPU scene now have one execution owner, reducing cross-thread lifetime ambiguity and keeping recovery adjacent to the resources it mutates.

The shell no longer duplicates Windows surface-recovery policy.

A deferred presentation cannot be mistaken for a successful frame, so retained planner revision/size state remains transactional.

Zero-sized/minimized windows do not require a platform-specific resize command in the generic worker protocol. No presentation is requested while the shell reports a zero client size; the retained worker state remains available for restore, and any stale/lost surface is recovered through the same deferred presentation path.

## Deferred

Later R3/R4 work may add:

- non-blocking compositor completion integration and deeper frame pipelining;
- explicit frame pacing/presentation feedback;
- broader multi-surface scheduling;
- process/IPC isolation and sandbox boundaries.

Those changes must preserve the backend-neutral frame contract and Windows adapter ownership established here.
