# ADR-0135: Platform-neutral accessibility tree foundation

Status: Accepted

## Context

R5 needs an accessibility representation that can later feed platform bridges without making Windows UI Automation, MSAA, COM objects, platform node identifiers, layout objects or native handles authoritative Web state. The DOM remains the semantic source of truth, while rendered fragment geometry is needed only to decide which connected nodes are currently exposed and to derive portable bounds.

Accessibility identity also has a different lifetime from one derived render snapshot. Rebuilding layout, changing geometry, or temporarily detaching and re-attaching the same DOM node must not create a new accessibility identity, while independent document lifecycles must not alias identities. At the same time, all retained identities, exposed nodes, fragment traversal and accessible-name storage must remain explicitly bounded.

## Decision

- `rarog-accessibility` is a portable derived-state crate. It depends only on Rarog-owned DOM, layout and geometry contracts and contains no Windows, UIA, MSAA, COM or native accessibility object types.
- `AccessibilityTreeState` represents one document lifecycle. It owns a fresh process-local scope, a monotonic non-zero serial allocator and a private exact `NodeId -> AccessibilityNodeId` correlation. Accessibility identities are never derived from platform IDs, Fragment IDs, LayoutNode IDs or pointers and are never reused within that state.
- Rebuilds stage identity changes in a preview map. The live retained identity map and next serial advance only after the complete snapshot is validated and constructed, so quota, name, geometry or consistency failures do not partially mutate retained accessibility identity state.
- An immutable `AccessibilityTree` snapshot records the source DOM generation, exact scoped root accessibility ID, current exposed-node lookup, semantic role/name/state data, optional portable `Rect` bounds, and parent/ordered-child accessibility relationships.
- A stale identity may remain privately retained by the lifecycle state while its source is detached or currently unrendered, but it does not resolve in a snapshot where that source is absent. Re-attaching the same `NodeId` within the same lifecycle restores the same accessibility identity.
- The DOM is semantic authority. `FragmentTree` contributes geometry only: one bounded traversal correlates fragment `dom_node` metadata back to exact DOM sources and unions finite, non-negative border boxes for each source. Fragment and layout identities are not accessibility authority.
- The document root is always the accessibility root. Non-root DOM nodes without rendered fragment geometry are omitted from the current snapshot. If an intermediate source is not exposed, an exposed descendant attaches to its nearest exposed DOM ancestor while preserving DOM traversal order.
- The bootstrap native-HTML role vocabulary is deliberately bounded: root web area, generic container, static text, button, link, heading, text field, checkbox, radio button, image, list and list item. Native element-role mapping is applied only to the HTML namespace; unsupported input types remain generic rather than receiving approximate text-field semantics. This is an architectural foundation, not HTML/ARIA accessibility-role completeness.
- The bootstrap accessible-name rules are deliberately bounded to selected Rarog-owned DOM inputs: normalized text, bounded `aria-label`, image `alt`, button-like input `value`, and normalized rendered-descendant text for selected native roles. Descendant text without current FragmentTree source geometry does not contribute. Name strings are accumulated under a per-node byte limit rather than constructed without bounds and checked afterward.
- The bootstrap state model contains only fixed engine-owned semantic values such as disabled, checked, expanded and focusable. A numeric non-negative `tabindex` may add focusability; malformed values do not.
- `AccessibilityLimits` explicitly bounds exposed nodes, retained identities, connected DOM nodes scanned, FragmentTree traversal, bytes per accessible name and aggregate accessible-name bytes. DOM/fragment worklists are bounded before enqueue, count/byte arithmetic is checked, node capacity is rejected before accessible-name work, and invalid/non-finite geometry fails closed.

## Consequences

- Future platform accessibility bridges can translate immutable Rarog accessibility snapshots without becoming DOM or Web authority.
- Accessibility node identity survives ordinary rebuilds and detach/re-attach within one document lifecycle while current snapshots still reject absent/stale nodes.
- Fragmentation can change without changing accessibility identity: multiple fragment border boxes for the same DOM source collapse into one portable accessibility bound.
- Resource pressure and malformed geometry/name input fail before retained identity state is committed.
- `AccessibilityTreeState` must be owned with the document lifecycle it represents. This slice does not introduce a new global document identifier or make one state safely interchangeable among unrelated `Document` instances.

## Non-goals

This decision does not add accessibility actions or events, mutation-to-accessibility incremental invalidation, platform event delivery, Windows UI Automation/MSAA providers, COM/native accessibility objects, platform accessibility handles or IDs as Web authority, complete ARIA role mapping, full Accessible Name and Description Computation, DOM/WebIDL accessibility APIs, WPT/accessibility conformance qualification, or any R6 work. Those remain later R5 Accessibility slices.
