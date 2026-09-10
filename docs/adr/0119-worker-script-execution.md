# ADR-0119: Dedicated worker script execution ownership

- Status: Accepted
- Date: 2026-09-10

## Context

R5 now has bounded worker identity and lifecycle ownership, but a worker identity is deliberately not a scheduler item, JavaScript realm, native thread or engine object. The next slice needs an execution owner for dedicated-worker tasks and microtasks without adding a second event-loop model or making SpiderMonkey types part of the portable Worker boundary.

Queued script source is attacker-influenced memory. It must be owned independently of caller lifetimes and bounded before being copied into a queue. Script/backend failures must also not leave scheduler work active indefinitely.

## Decision

`rarog-workers` composes the existing `rarog-scheduler::EventLoopScheduler` and `rarog-script::ScriptRuntime` contracts. A `WorkerExecution` is bound to one exact `WorkerId`, borrows one Script runtime exclusively for its lifetime, creates one exact Rarog `RealmId`, and owns that realm's lifetime until explicit shutdown or drop.

Construction and every queue/checkpoint/execution-driving operation also requires the exact `WorkerRegistry` and verifies that the bound worker is still present and `Running` before touching queued or executable work. A foreign, retired, `Created`, or `Closing` identity is rejected; `WorkerId` remains correlation only and never becomes execution authority. The registry is borrowed per operation rather than for the execution lifetime, so lifecycle transitions can revoke execution immediately.

Tasks and microtasks carry owned script text only. The configured `ScriptRealmLimits::max_source_bytes` limit is checked before the text is copied into scheduler storage. Queue-count authority remains entirely in `SchedulerLimits`; Worker execution does not invent a parallel queue or event loop.

`WorkerExecution::next_step` delegates ordering to `EventLoopScheduler`. Task completion therefore requests the scheduler's existing microtask checkpoint and queued microtasks drain before the next task. An explicit checkpoint can also be requested when no task caused one.

All evaluation is through `ScriptRuntime::evaluate` using the worker-owned Rarog realm. A JavaScript throw remains an `EvaluationOutcome` completion. If evaluation returns a Script/backend error, the exact scheduler work item is completed before the error is propagated so the worker cannot become permanently wedged on stale active work.

Explicit `shutdown` destroys the exact realm and reports teardown errors. Drop performs best-effort realm destruction and drops the scheduler with any pending owned payloads, so destroying the execution owner cannot execute queued work afterward. No SpiderMonkey, Host, Site-process, OS-thread or platform-handle type enters this crate.

## Validation

Coverage demonstrates:

- construction and execution-driving require the exact registry and a live `Running` worker;
- `Closing`, retired and foreign-registry worker identities cannot queue or execute script work;
- execution identity remains bound to its exact worker and realm;
- queued source remains owned after the caller mutates its original buffer;
- task completion uses the existing microtask-checkpoint ordering before the next task;
- explicit checkpoints run microtasks without a preceding task;
- task and microtask queue limits remain scheduler-owned;
- oversized source is rejected before queue insertion;
- Script/backend evaluation failure completes scheduler work and does not wedge later tasks;
- JavaScript throws remain Script completions;
- explicit shutdown destroys the exact realm once;
- dropping execution discards pending work and destroys its realm without evaluating pending payloads.

## Consequences

R5 now has a portable dedicated-worker execution owner that composes existing scheduler and Script abstractions while preserving bounded memory and exact realm lifetime.

This ADR does not implement Worker message delivery or structured clone, Service Worker registration/scope/lifecycle, Fetch interception, Host/navigation-context wiring, OS thread/process isolation, SharedWorker semantics, or any R6 compatibility qualification. Those remain separate R5 slices.
