# ADR-0120: Bounded Worker message delivery and structured payload ownership

- Status: Accepted
- Date: 2026-09-10

## Context

R5 already has bounded Worker identity/lifecycle ownership and a dedicated execution owner composed from the existing event-loop scheduler and Script API. Worker messaging must add owned data transfer without turning `WorkerId` into authority, inventing another event loop, exposing SpiderMonkey objects, or allowing attacker-controlled payloads to grow without explicit bounds.

A queued message can outlive the caller's source value and can remain pending while the destination scheduler is backpressured. Its complete retained lifetime therefore has to stay charged to a bounded mailbox, and lifecycle changes must be able to revoke access before delivery even after a message was selected for scheduling.

## Decision

`rarog-workers` owns an intentionally bounded structured message value subset: null, boolean, exact floating-point bits, UTF-8 strings, byte vectors, arrays and string-keyed objects. This is an internal R5 data contract, not a claim of Web structured-clone completeness. Payload measurement is iterative and bounded by explicit byte, item and depth limits plus a hard maximum traversal depth. Aggregate arithmetic is checked and the per-message and remaining mailbox budgets are validated before the caller's value is cloned into durable mailbox ownership.

`WorkerMessageMailbox` owns queued payloads and is bounded by both message count and retained payload bytes. A `WorkerMessageId` is a scoped monotonic non-zero correlation identity only. The supported routes are deliberately narrower than arbitrary `WorkerId` addressing: an exact external root owner may communicate with its root worker, and directly related parent/child workers may communicate with each other. Enqueue and payload access use the exact `WorkerRegistry`; required endpoints must still be present and `Running`, and retained ownership relationships are revalidated before delivery. Foreign, retired, `Created` and `Closing` identities therefore cannot use a numeric worker reference as message authority.

Worker-targeted delivery is handed into the existing `WorkerExecution` / `EventLoopScheduler`. The scheduler owns only a `WorkerMessageId` task marker while the mailbox continues to own and charge the payload. Selecting a message binds it to the exact scheduler `TaskId`; the public `WorkerMessageDelivery` token is valid only for the exact worker and currently active task. A second execution object cannot take a message that is already marked for another scheduler task. The oldest worker-targeted message remains the only schedulable message until its delivery completes, preserving FIFO and single-delivery semantics without a second event loop.

`message_for_delivery` revalidates the active delivery token and current Worker route/lifecycle before returning a borrowed message. Completing delivery removes the exact mailbox item, releases its charged bytes, then completes the exact scheduler task so the existing task-to-microtask checkpoint semantics remain authoritative. Scheduler backpressure leaves the payload pending and charged rather than dropping it or growing another queue.

Lifecycle revocation makes a selected payload unreadable even when its scheduler marker already exists. Cleanup is intentionally allowed to remove that inaccessible item and complete/drop the corresponding work without restoring Worker authority. Explicit `discard_for_worker` and `shutdown_with_mailbox` recover charged capacity. If an execution owner is dropped while a delivery is selected, the mailbox marker remains fail-closed rather than allowing automatic duplicate redelivery; explicit lifecycle/mailbox cleanup is required to reclaim it.

No SpiderMonkey, Host, Site-process, OS-thread, process-handle or backend-handle type enters the portable message contract.

## Validation

Coverage demonstrates:

- invalid limits fail before mailbox creation;
- structured payload bytes, items and depth are bounded before durable ownership;
- payload measurement is iterative and checked;
- caller mutation after enqueue cannot alter the retained payload;
- queue message and byte limits provide deterministic backpressure and capacity recovery;
- message identities are scoped, monotonic correlation values;
- root-owner and direct parent/child routes are accepted while unrelated, self, foreign and stale routes fail closed;
- non-running and retired workers cannot enqueue or receive message data;
- worker messages enter only the existing execution scheduler;
- scheduler backpressure leaves mailbox data pending and charged;
- an active delivery retains its payload until exact completion;
- only the oldest message for a worker can be scheduled until it completes;
- completion preserves the scheduler's existing microtask checkpoint behavior;
- lifecycle revocation after selection blocks payload access while cleanup can still release capacity;
- dropping an execution owner cannot cause a selected message to be automatically redelivered by a replacement execution.

## Consequences

R5 now has a bounded portable ownership and scheduling foundation for dedicated-Worker messages without widening process/Host authority or duplicating event-loop semantics.

This ADR does not implement script-visible `MessageEvent` construction, a JavaScript serialization/deserialization adapter, transferables, `MessagePort`, SharedWorker semantics, generic Web structured-clone completeness, Service Worker registration/scope/lifecycle, Fetch interception, Host/navigation-context wiring, OS thread/process isolation, or any R6 compatibility qualification. Those remain separate work.
