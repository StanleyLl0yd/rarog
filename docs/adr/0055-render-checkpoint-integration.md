# ADR-0055: Retained rendering at event-loop checkpoints

Status: accepted

## Context

R2 now has independent DOM mutation journaling, retained rendering and task/microtask scheduling. `rarog-dom` records every successful mutation with a monotonically increasing document generation. `RenderSession::update()` already consumes those records through the CSS invalidation set and selects the existing retained paint, subtree, flow, geometry or full-rebuild path.

A second script-specific dirty tree or mutation queue would duplicate this machinery and risk making script-originated changes behave differently from host-originated DOM changes. Rendering immediately after every binding call would also violate task/microtask batching and perform unnecessary intermediate work.

## Decision

`rarog-engine` owns a thin `EngineEventLoop<T, M>` wrapper around `rarog-scheduler::EventLoopScheduler<T, M>`.

Task and microtask payloads remain generic and are returned to the caller for execution. Script bindings, host callbacks or other engine code may mutate the existing `RenderSession` document while that work item is active. The wrapper does not interpret payloads and stores no JavaScript values or callbacks.

When the scheduler emits `MicrotaskCheckpointComplete`, the engine wrapper synchronously calls the existing `RenderSession::update()` exactly once and returns `EngineEventLoopStep::RenderCheckpoint(IncrementalReport)`. All DOM mutations performed by the completed task and every microtask drained in that checkpoint are therefore consumed together through the existing document generation journal and retained invalidation pipeline.

A checkpoint with no DOM mutations still goes through `RenderSession::update()` and reports `IncrementalMode::Unchanged`. This keeps the first contract deterministic and gives the engine one observable checkpoint result. A render failure is propagated as an engine-event-loop error and is considered fatal for that progression attempt; the scheduler does not provide render retry semantics in this slice.

The wrapper forwards queueing, completion, cancellation and checkpoint requests rather than reimplementing scheduler state.

## Consequences

Script-originated and host-originated DOM mutations share one mutation journal and one retained rendering implementation. No SpiderMonkey type enters `rarog-engine`, and `rarog-scheduler` remains independent of DOM and rendering.

Rendering is deferred until all microtasks in the current checkpoint have completed, so intermediate task/microtask DOM states are not rasterized. Multiple changes to the same or related nodes can be coalesced by the existing invalidation machinery.

This does not yet expose DOM objects or methods inside JavaScript. Generated WebIDL bindings and concrete callback invocation will use this checkpoint contract later; they must mutate the same `RenderSession` document rather than bypass its journal.

Browser rendering opportunities, animation-frame timing, timers, compositor scheduling and multi-document event loops remain later work.
