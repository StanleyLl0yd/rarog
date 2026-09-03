# ADR-0053: Rarog-owned event dispatch boundary

Status: accepted

## Context

R2 needs Event and EventTarget semantics before DOM bindings, input adapters and script callbacks can be connected. Putting JavaScript callback values directly into `rarog-dom` would make the DOM depend on the selected script engine and would conflict with the replaceable runtime boundary established by ADR-0050 through ADR-0052.

EventTarget is also broader than DOM nodes. Window-like globals, network-facing objects and later platform objects need the same listener and dispatch semantics, so the foundation should not be coupled to `NodeId` or to the DOM arena.

## Decision

Rarog owns event registration and dispatch semantics in the dependency-free `rarog-events` crate.

`EventTargetRegistry<T>` is generic over an engine-owned target identity. The host of a dispatch supplies the target and its ordered ancestor path. For DOM dispatch, the engine can derive that path from `rarog-dom`; non-DOM EventTargets can supply a different path without changing the event crate.

Listener callbacks are represented only by opaque `EventListenerId` values. `rarog-events` never stores `JS::Value`, SpiderMonkey pointers, `RootedValueId`, DOM wrapper pointers or executable closures. A script/bindings layer owns the mapping from an opaque listener identity to its rooted callback and performs the actual callback invocation after `next_listener` returns an owned invocation record.

Listener identities are process-unique across allocators through a private allocator scope plus local serial. Registration identity is separate from callback identity so removing and re-adding a callback cannot accidentally resurrect a stale dispatch snapshot.

The first dispatch state machine owns:

- capture traversal from root toward the target;
- at-target capture and non-capture listener phases;
- optional bubbling from the target parent toward the root;
- duplicate suppression by event type, callback identity and capture flag;
- listener removal using the same matching key;
- `once` removal before callback invocation, including recursive dispatch safety;
- passive-listener suppression of `preventDefault()`;
- `stopPropagation()` and `stopImmediatePropagation()` semantics;
- mutation-safe listener snapshots for the current target/phase, with liveness revalidation before each invocation.

`next_listener` does not retain a borrow across callback execution. This lets the bindings layer invoke script and then add/remove listeners or update dispatch flags before asking for the next listener. A listener removed after a snapshot was created is skipped. A listener newly added to the active snapshot group is deferred rather than appended to that snapshot.

The ancestor slice passed to `EventDispatch::new` is ordered from the target's immediate parent outward to the root. The event crate reverses that sequence for capture and uses it directly for bubbling.

## Consequences

DOM and script crates remain independent of the concrete JavaScript backend. Event semantics can be tested without SpiderMonkey, a window system or a DOM tree, while real script callbacks can later use the same dispatch cursor.

A dispatch owns its propagation/default-prevention state. `EventListenerInvocation` carries current-target, phase and passive metadata for the bindings layer, but no executable callback object.

The first slice deliberately does not model Shadow DOM retargeting/composed paths, trusted-event provenance, browser default actions, activation behavior, timestamps or UI-event subclasses. Those require additional Web-platform state and will extend this Rarog-owned boundary rather than bypass it.
