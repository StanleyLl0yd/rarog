# ADR-0137: Deterministic accessibility invalidation and committed-snapshot refresh

Status: Accepted

## Context

ADR-0135 established the platform-neutral accessibility tree and lifecycle-scoped stable identities. ADR-0136 added bounded semantic actions and events. After either DOM mutation or render geometry change, however, an accessibility snapshot can no longer be treated as current merely because its identities remain valid. R5 therefore needs one deterministic refresh boundary that follows the existing DOM mutation journal and committed render state without creating a second source of DOM, layout or platform authority.

The refresh path also has to remain safe under failure. Accessibility is derived state: a tree/name/event budget failure must not make rendering fail, and event backpressure must not partially publish a new snapshot or consume fresh retained identities. Conversely, repeatedly retrying an event batch that can never fit in the configured bounded queue would leave accessibility permanently stale.

## Decision

- `AccessibilitySnapshot` is current only for the exact source DOM generation and an engine-owned non-zero geometry revision. The geometry revision advances after committed relayout/full-rebuild or viewport-resize geometry changes; Fragment and Layout identities remain non-authoritative implementation details.
- `AccessibilityRuntime::refresh` rejects regressing DOM generations and geometry revisions before candidate publication. A refresh clones the lifecycle `AccessibilityTreeState`, builds a complete candidate tree, validates scope/stable identity correlation, stages the semantic event diff, and only then commits the preview state and snapshot.
- Candidate diffing is deterministic. Membership, role, parent or child-order changes imply `TreeChanged`; retained-node name, state and bounds changes produce fixed `NameChanged`, `StateChanged` and `BoundsChanged` events carrying the candidate DOM generation. Stable source nodes must retain the same `AccessibilityNodeId`; an unexpected identity change fails closed.
- The event queue remains bounded and FIFO. A detailed batch is published when it fits. If the detailed batch cannot fit but at least one queue slot is available, the entire refresh notification is conservatively coalesced to one root `TreeChanged` event. If no slot is available, refresh fails before the candidate tree/identity state is committed. This makes large refreshes eventually publishable without unbounded notification retention.
- `rarog-engine::RenderSession` is the orchestration owner. It derives accessibility invalidation from the same DOM mutation journal/checkpoint already used by render invalidation, before the render update prunes consumed mutation records. Structural mutations request tree/semantic/bounds refresh; text mutations request semantic/bounds refresh; accessibility-relevant attributes request semantic refresh and geometry/membership-sensitive attributes conservatively add tree/bounds refresh.
- Accessibility refresh runs only after the ordinary render update or resize has committed its new DOM/layout/Fragment state. Subtree/flow/geometry relayout advances the geometry revision and requests tree/bounds refresh; a full render rebuild requests a full accessibility rebuild. A snapshot is exposed to consumers only when both its DOM generation and geometry revision exactly equal the committed engine state.
- Accessibility remains derived and non-authoritative. Initial accessibility construction or later refresh failure does not make `RenderSession` construction/update/resize fail. The engine retains a bounded coalesced pending invalidation plus the fixed Rarog error classification, exposes no stale snapshot as current, and retries after later committed state changes. A failed initial build can be reconstructed from the current committed DOM/Fragment state.
- Geometry-revision exhaustion fails closed by making the accessibility snapshot unavailable and retaining a pending full rebuild rather than wrapping or reusing a revision value.
- The refresh path imports no Windows accessibility/native types and does not create a second platform event stream. The existing `AccessibilityEvent` queue remains the portable semantic-notification boundary for the later platform bridge.

## Consequences

- DOM and Fragment state remain authoritative; accessibility snapshots are immutable derived views tied to exact committed generations.
- Stable accessibility identities survive deterministic rebuilds, detach/re-attach and geometry changes within one document lifecycle, while stale snapshots are never presented as current.
- Tree/identity publication is atomic with respect to tree-build failures and event backpressure.
- Large mutation batches cannot permanently wedge accessibility merely because their detailed event diff exceeds the configured queue capacity; conservative root-tree invalidation provides bounded recovery.
- Accessibility resource limits cannot deny rendering service. They can make accessibility temporarily unavailable, which is observable through the engine's current/pending/error state and recoverable through later refresh.
- The existing DOM mutation journal remains the sole mutation-order authority; no duplicate accessibility mutation log is introduced.

## Non-goals

This decision does not add Windows UI Automation/MSAA providers, COM/native accessibility objects, platform handles or provider IDs as Web authority, broad ARIA or Accessible Name completeness, incremental subtree-only accessibility tree construction, DOM/WebIDL accessibility APIs, assistive-technology compatibility claims, WPT qualification, WebDriver/BiDi qualification, real-Web compatibility qualification, or any R6 work. The first Windows accessibility bridge remains the next R5 Accessibility slice.
