# ADR-0118: Bounded worker identity and lifecycle foundation

- Status: Accepted
- Date: 2026-09-10

## Context

R5 needs a portable Worker ownership and lifetime foundation before dedicated worker event loops, script realms, message delivery or Service Worker registration can be integrated. Worker identity is a Web/runtime identity domain: it must not be inferred from a Site-process identity, OS PID, thread handle, JavaScript-engine object or scheduler work ID.

Workers may own nested workers, so the ownership graph itself is attacker-influenced state. Live-worker count, direct fan-out and ownership depth therefore require explicit limits, and teardown must make stale identities unusable without recursive unbounded traversal.

The worker foundation also must not depend on `rarog-host`. Host/navigation-context integration will supply an appropriate root-owner identity in a later slice; reversing that dependency would make a lower-level Web runtime boundary depend on Host control-plane implementation details.

## Decision

Rarog owns worker identity, ownership and lifecycle in the dependency-free `rarog-workers` crate.

`WorkerRegistry<O>` is generic over an embedder/Host-selected root-owner type `O`. A worker is owned either directly by one `WorkerOwner::Root(O)` or by one live parent `WorkerId`. The registry does not contain Host, Site-process, OS, SpiderMonkey, scheduler-payload or backend handle types.

Worker identities have two non-zero components: a process-local registry scope and a monotonic serial within that registry. Independent registries therefore cannot accidentally alias their normal worker identities, while retirement never reuses an earlier serial in the same registry. Identity or scope exhaustion fails closed. `WorkerId` is an identity/reference only; possession of a value does not establish Host or platform authority.

The registry has three independent limits. Defaults are 256 live workers, 32 direct children per owner and ownership depth 8. Zero-valued limits are invalid. Root owners and parent workers consume the same direct-child bound conceptually, while the live-worker bound applies across the whole registry.

The initial live lifecycle is explicit:

- `Created` — identity and ownership exist, but execution is not yet running;
- `Running` — the worker may own newly created child workers;
- `Closing` — shutdown has begun and the worker cannot return to `Running` or create new children.

Only `Created -> Running` is an ordinary start transition. `begin_close` moves the selected worker and every live descendant to `Closing`. `retire` requires the selected worker to be `Closing`, then removes its whole descendant subtree and releases the parent's direct-child budget. Retired identities become unknown/stale immediately.

Root-owner destruction is a stronger ownership teardown boundary. `retire_root_owner` removes every worker rooted directly in that owner and all descendants regardless of the workers' current live state. This models destruction of the external owner itself rather than an ordinary worker lifecycle transition; unrelated root owners remain intact.

Ownership traversal is iterative and bounded by the configured live-worker limit. `BTreeMap`/`BTreeSet` storage gives deterministic traversal order without making order itself Web-visible semantics.

## Validation

Coverage must demonstrate:

- rejection of zero limits and zero worker identity components;
- root-owner identity and depth retention;
- independent registries do not alias worker identities;
- retired serials are not reused and serial exhaustion fails closed;
- global live-worker, direct-child and ownership-depth limits fail before creating another worker;
- only a running parent may create a child;
- invalid lifecycle transitions are rejected;
- closing a parent cascades to all descendants and prevents restart;
- ordinary retirement requires `Closing` and releases parent child capacity;
- root-owner teardown is scoped to that owner and removes all descendants;
- stale/retired identities are rejected.

## Consequences

R5 now has a bounded portable identity and ownership tree on which dedicated worker execution and messaging can be built without conflating Web workers with processes, threads, scheduler items or JavaScript-engine objects.

This ADR does not implement or claim Worker/Web Worker API completeness, worker task or microtask execution, script-runtime ownership, structured-clone/message delivery, SharedWorker semantics, Service Worker registration/scope/lifecycle, Fetch interception, OS thread/process isolation or any R6 compatibility qualification. Those remain separate R5 slices.