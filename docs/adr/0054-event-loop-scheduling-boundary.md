# ADR-0054: Rarog-owned event-loop scheduling boundary

Status: accepted

## Context

R2 needs task and microtask ordering before script callbacks, DOM mutations, timers and network completions can be connected. Scheduling is an engine concern: if the queue directly stores SpiderMonkey values, closures, platform handles or DOM references, the selected script runtime and host adapters would leak into the event-loop core.

The scheduler also needs an explicit completion protocol. Merely popping a task from a queue is insufficient because the engine must know when that task has finished before it may perform the required microtask checkpoint and advance to the next task.

## Decision

Rarog owns event-loop ordering in the dependency-free `rarog-scheduler` crate.

`EventLoopScheduler<T, M>` is generic over engine-owned task and microtask payloads. The scheduler does not execute those payloads and does not store closures, `RootedValueId`, SpiderMonkey values, DOM pointers, platform handles or network backend objects. Engine/script integration owns the meaning and execution of each payload.

Tasks and microtasks receive scheduler-scoped opaque identities. Scheduler instances use process-unique private scopes plus local serials, so work identities cannot alias across independently constructed schedulers while normal queue operations avoid a global atomic allocation for every work item.

The first scheduler contract is stateful:

- tasks are selected FIFO;
- microtasks are selected FIFO within a checkpoint;
- only one work item may be active at a time;
- the caller must complete the exact active work identity before another work item can be selected;
- completing a task makes a microtask checkpoint mandatory before the next task;
- microtasks queued while that checkpoint is being drained join the same checkpoint;
- an empty checkpoint produces an explicit `MicrotaskCheckpointComplete` step before task selection resumes;
- an embedder may explicitly request a checkpoint when no task completion triggered one;
- task and microtask queue limits include active work so dequeueing cannot temporarily bypass configured capacity;
- queued tasks can be cancelled, while active work cannot be removed through queue cancellation.

`TaskSource` classifies task provenance without encoding backend-specific objects. The initial source set is intentionally small and can be extended when concrete R2 subsystems are connected.

## Consequences

The engine can drive task execution, JavaScript jobs and later platform/network completions through one deterministic ordering contract without making the scheduler depend on SpiderMonkey or DOM internals.

The explicit completion barrier makes reentrancy visible: a caller cannot accidentally ask for the next task while the current task or microtask is still executing. The explicit checkpoint-complete step gives the engine a stable hook for later rendering/invalidation work after microtasks have drained.

This slice does not implement wall-clock timers, delayed-task heaps, task-source prioritization, rendering opportunities, nested event loops, animation frames, idle callbacks or worker event loops. Those features will extend the Rarog-owned scheduling boundary rather than bypass it.
