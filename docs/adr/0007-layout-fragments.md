# ADR-0007: Separate DOM, layout-node and fragment identities

- Status: Accepted
- Date: 2026-08-26

## Context

A DOM node is mutable script-visible state. Layout state is derived from DOM + style, and rendered geometry may fragment further.

Treating one DOM node as one permanent layout object would make incremental layout, anonymous boxes, inline fragmentation, multicolumn/page fragmentation, process isolation and crash recovery harder.

Paint also needs a stable snapshot boundary rather than direct access to mutable DOM/layout internals.

## Decision

Rarog uses three distinct identity domains:

```text
NodeId → LayoutNodeId → FragmentId
```

`LayoutNodeId` and `FragmentId` are separate Rust types. Numeric equality between them and `NodeId` has no semantic meaning.

The Layout Tree is disposable derived state. The Fragment Tree is a disposable geometry snapshot derived from the Layout Tree.

Paint consumes the Fragment Tree and emits a display list. Layout does not draw directly.

R0 may produce approximately one fragment per layout node, but APIs must not depend on that relationship remaining one-to-one.

## Consequences

- DOM lifetime is independent of layout/fragment lifetime.
- Future anonymous layout nodes do not require fake DOM nodes.
- One layout node can later produce multiple fragments.
- Paint/compositor work can consume snapshots without mutating DOM.
- Rebuilding layout/fragments after invalidation or process recovery remains architecturally valid.
