# ADR 0013: DOM ownership and future script-wrapper lifetime

Status: Accepted for R1/R2 preflight

## Context

Rarog currently stores DOM nodes in a `Document`-owned arena and exposes opaque `NodeId` values. R2 will add WebIDL bindings and SpiderMonkey wrappers. The native ownership contract must be fixed before wrappers, detached subtrees, event listeners, and script-created nodes make lifetime bugs expensive to unwind.

## Decision

### Native ownership

- `Document` owns the native node arena for the lifetime of that document.
- `NodeId` is meaningful only together with its owning `Document`; it is not a process-global identity and must never be used as a raw pointer surrogate.
- Node IDs are not reused while a `Document` is alive.
- Detaching a node changes connectedness, not native liveness. A detached node remains arena-owned until an explicit future collection policy proves it unreachable.
- `RenderSession` owns the active `Document` and the derived style, layout, display-list, damage, and framebuffer state. Derived state may be rebuilt without changing DOM identity.

### Script wrappers

Future SpiderMonkey DOM wrappers will not own native nodes individually and will not store direct pointers into the DOM arena. A wrapper payload must resolve through a document/realm owner plus opaque `NodeId` (or an equivalent checked handle with the same semantics).

The wrapper registry is per document/realm. It provides stable wrapper identity for a live native node and is the only place allowed to map native identities to JavaScript objects.

A wrapper lookup must fail safely when its document/realm is no longer live. Cross-document adoption, if added later, must explicitly remap ownership and wrapper identity; copying a `NodeId` between documents is invalid.

### Detached subtrees and future collection

R1/R2 may create large detached subtrees. Connectedness must therefore remain separate from liveness and render invalidation. Detached mutations must not trigger connected render work.

Native reclamation of detached nodes is deliberately deferred until the runtime can prove all of the following:

1. the subtree has no connected DOM owner;
2. no live JavaScript wrapper or native handle can resolve to it;
3. no event/listener/task queue owns a reference requiring it to stay observable.

Until that proof exists, retaining detached nodes in the document arena is preferred over introducing use-after-free risk.

### Realm and document teardown order

Document teardown must be ordered so JavaScript finalization can never observe freed native DOM storage:

1. stop new script tasks and DOM mutations for the realm;
2. cancel or drain document-owned callbacks/tasks according to the event-loop contract;
3. sever listener/callback roots owned by the document;
4. clear wrapper-registry mappings and destroy/finalize the SpiderMonkey realm;
5. only then drop the native `Document` arena and its derived render state.

Shutdown code must remain idempotent and must not depend on JavaScript finalizers mutating an already-tearing-down document.

## Consequences

- No raw DOM arena pointers may cross the WebIDL/SpiderMonkey boundary.
- DOM read APIs remain checked/fallible for foreign or stale identities.
- Connectedness-aware invalidation is part of the lifetime model rather than an optimization detail.
- Future detached-node collection requires an explicit design change and tests covering wrapper reachability and teardown order.
- R2 bindings can choose the concrete SpiderMonkey rooting/tracing types without changing the native identity contract established here.
