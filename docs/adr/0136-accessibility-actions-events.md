# ADR-0136: Bounded accessibility actions and semantic events

Status: Accepted

## Context

ADR-0135 established a platform-neutral accessibility tree whose identities are scoped Rarog correlations over DOM authority. R5 next needs a way for an accessibility consumer to request a small set of semantic actions and receive bounded semantic notifications without turning accessibility snapshots, platform provider objects, DOM `Event` instances or backend handles into Web authority.

Action delivery also crosses a mutation boundary. A stale or foreign accessibility identity must not reach an executor, and event backpressure must be established before an action can cause a DOM or host-side effect. DOM-backed state mutations need a narrow target-scoped seam so a replaceable executor cannot acquire unrestricted accessibility or platform authority from this layer.

## Decision

- `rarog-accessibility` owns a fixed bootstrap `AccessibilityAction` vocabulary: focus, invoke, toggle and set-expanded. The internal executor command resolves toggle to an exact checked value before execution.
- Actions are methods on the exact `AccessibilityTreeState` lifecycle. Processing rejects a tree from another lifecycle scope, a target from another scope, a snapshot whose DOM generation is stale, a target absent from the current snapshot, a disconnected source, or a snapshot whose role/state no longer matches the exact live DOM source.
- Action eligibility is derived only from the current Rarog role/state plus exact live DOM state. Disabled targets are rejected. Focus requires current focusability, invoke is limited to the bootstrap button/link roles, toggle is limited to checkboxes, and set-expanded requires an existing expanded state.
- `AccessibilityActionExecutor` receives only the exact DOM `NodeId`, a fixed action command and a target-scoped staging object. The staging object can request only checked or expanded mutations on that exact source. It exposes no arbitrary DOM mutation API, native handle, platform accessibility identifier, COM pointer or backend error object.
- DOM-backed executor mutations are staged rather than applied immediately. An executor error or executor-contract violation therefore discards staged DOM changes. After a successful executor result, the accessibility layer applies the exact staged target mutation through the ordinary Rarog DOM mutation APIs.
- Focus/invoke remain replaceable executor operations because the current DOM contract does not own focus or activation authority. The built-in DOM executor deliberately reports them unavailable rather than manufacturing approximate script/event behavior.
- `AccessibilityEvent` is a fixed engine-owned semantic notification containing only the exact accessibility correlation ID, DOM source ID, event kind and resulting DOM generation. It is not a DOM `Event` and does not participate in capture/bubble/listener dispatch.
- `AccessibilityEventQueue` is a non-zero bounded FIFO. Capacity is checked with overflow-safe arithmetic before an action reaches the executor. A successful action that requires an event consumes the previously validated slot; queue backpressure therefore fails before executor or DOM mutation side effects.
- The bootstrap semantic event vocabulary is focus-changed, invoked, state-changed, name-changed, value-changed and tree-changed. This slice emits only action-result notifications. Automatic DOM/layout-to-accessibility invalidation and event production remain a later R5 slice.
- Fixed Rarog-owned error classifications cover lifecycle/scope mismatch, stale snapshots, missing/live-source failures, unsupported actions, event backpressure, executor failures, executor-contract violations and DOM mutation failure. Native/backend error values never cross the contract.

## Consequences

- Platform bridges can translate a validated action into a replaceable executor without gaining DOM or Web authority from a platform accessibility object.
- Event queue pressure is deterministic and cannot cause an action to mutate first and discover notification loss afterward.
- Checked/expanded DOM changes are atomic with respect to executor failure because they are committed only after successful executor validation.
- A DOM mutation performed by a successful action advances the ordinary DOM generation and makes the old accessibility snapshot stale until the existing tree builder is run again. This slice intentionally does not add automatic invalidation.
- Focus/invoke integration remains an engine/platform responsibility behind the executor seam; no synthetic DOM event or script-dispatch shortcut is introduced here.

## Non-goals

This decision does not add automatic DOM/layout-to-accessibility invalidation, platform event delivery, Windows UI Automation/MSAA providers, COM/native accessibility objects, platform handles or IDs as authority, broad ARIA action completeness, DOM/WebIDL accessibility APIs, synthetic DOM event dispatch, WPT/accessibility conformance qualification, or any R6 work. Those remain later R5 slices where applicable.
