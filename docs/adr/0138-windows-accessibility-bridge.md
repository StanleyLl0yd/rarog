# ADR-0138: HWND-bound Windows accessibility bridge without platform authority

Status: Accepted

## Context

ADR-0135 established Rarog-owned lifecycle-scoped accessibility identities, ADR-0136 bounded semantic actions/events, and ADR-0137 tied accessibility snapshots to exact committed DOM and geometry generations. R5 now needs the first Windows host bridge while preserving those authority boundaries. A Windows provider correlation, HWND, WinEvent value or future native provider object must never become Web state or substitute for `AccessibilityNodeId`, DOM source identity, document generation or geometry revision.

The bridge must also fail safely when platform state disappears or backpressure occurs. Window handles can become invalid after attachment, provider callbacks can arrive after nodes are retired, and native notification delivery can fail independently of Web state. None of those conditions may mutate DOM through a stale target, expose an old platform snapshot as current, grow queues without bound, or turn a native error value into Web semantics.

## Decision

- `rarog-platform` owns a Windows-free immutable snapshot/event/action projection. It contains only fixed Rarog semantic values, Rarog correlation IDs, exact document generation and geometry revision. Projection validation proves one rooted, reciprocal, duplicate-free, acyclic/reachable tree before a platform backend may accept it.
- `rarog-platform-windows` owns private monotonic provider serials and bounded provider/event/action state. Provider serials are correlation only, are never exported as Web authority, and are not reused after retirement. Snapshot replacement preserves correlations only for retained current nodes and retires removed correlations deterministically.
- Native action callbacks fail closed unless the provider correlation still resolves and the callback's lifecycle/document/geometry context exactly matches the current snapshot. Only then is a fixed `PlatformAccessibilityActionRequest` queued. `rarog-engine` revalidates that request against the current Rarog snapshot and delegates to the ADR-0136 action path before any DOM mutation.
- The desktop shell owns the real winit window and extracts its Win32 HWND. `WindowsPlatformHost` advertises no accessibility capability until that HWND is attached successfully. HWND never crosses into engine/core Web crates.
- `rarog-platform-windows-native` validates the HWND with `IsWindow` both when the bridge is attached and before every native event publication. The native layer maps the fixed semantic notification vocabulary to WinEvent constants and publishes against the attached HWND. Window loss therefore becomes a fixed native failure rather than an apparent successful delivery.
- Snapshot replacement is ordered before event publication. If replacement fails, the engine attempts to clear the old platform snapshot so stale platform-derived state is not presented as current. Native notification failure cannot roll back or mutate Rarog DOM/layout/accessibility state.
- Provider maps and pending native-event/action queues have explicit non-zero limits. Retry pressure remains bounded; when detailed native notifications cannot be retained, the Windows layer may conservatively coalesce them to a root `TreeChanged` notification rather than grow without bound.
- Native errors are translated to fixed Rarog/platform classifications. Raw HWND values, WinEvent constants and Windows-native error codes are confined below the platform/native boundary.

## Consequences

- Rarog DOM plus committed accessibility snapshot generations remain authoritative; Windows state is a disposable derived projection.
- Stale or foreign provider callbacks cannot select a DOM target merely because an OS-side correlation still exists.
- Destroyed-window notification attempts fail closed and use the same bounded failure path as other native publication failures.
- The shell can share one late-bound `WindowsPlatformHost` with the engine while keeping accessibility disabled until a real window exists.
- Linux portability remains possible because Windows native operations are isolated behind target-specific code and the portable contract imports no Windows types.

## Non-goals

This decision is the first HWND-bound Windows accessibility bridge foundation. It does not claim complete UI Automation or MSAA provider-object/pattern implementation, broad native accessibility pattern support, broad ARIA/Accessible Name/WebIDL completeness, assistive-technology compatibility certification, WPT qualification, WebDriver/BiDi qualification, real-Web compatibility qualification, signed compatibility profiles, or any R6 work. Real-machine/native-provider certification requires separate later evidence and must not be inferred from WinEvent notification publication or the bounded callback seam.
