# ADR-0138: HWND-bound Windows UI Automation provider bridge without platform authority

Status: Accepted

## Context

ADR-0135 established Rarog-owned lifecycle-scoped accessibility identities, ADR-0136 bounded semantic actions/events, and ADR-0137 tied accessibility snapshots to exact committed DOM and geometry generations. R5 now needs the first real Windows UI Automation provider bridge while preserving those authority boundaries. A Windows provider object, HWND, runtime ID or private provider serial must never become Web state or substitute for `AccessibilityNodeId`, DOM source identity, document generation or geometry revision.

The bridge must also fail safely when native objects outlive the snapshot that created them. UI Automation can retain COM providers after Rarog has replaced or cleared its derived accessibility state, action callbacks can arrive late, and native event delivery can fail independently of Web state. None of those conditions may select a stale DOM target, expose an old platform snapshot as current, grow queues without bound, or turn a native error into Web semantics.

## Decision

- `rarog-platform` owns a Windows-free immutable snapshot/event/action projection. It contains only fixed Rarog semantic values, Rarog correlation IDs, exact document generation and geometry revision. Projection validation proves one rooted, reciprocal, duplicate-free, fully reachable tree before a platform backend may accept it.
- `rarog-platform-windows` owns private monotonic provider serials plus bounded provider/event/action state. Serials are non-zero, never exceed `i32::MAX`, are never exported as Web authority and are never reused after retirement. Snapshot replacement preserves correlations only for retained current Rarog nodes and commits wrapper correlation state only after native replacement succeeds.
- `rarog-platform-windows-native` binds the bridge to the shell-owned Win32 HWND and installs the narrow `WM_GETOBJECT`/`UiaReturnRawElementProvider` entry point. The minimum provider foundation implements `IRawElementProviderSimple`, fragment/root navigation, runtime IDs, hit testing/focus, and only the currently supported Invoke, Toggle and ExpandCollapse patterns. Raw HWND/COM/SAFEARRAY/VARIANT operations stay inside this native crate.
- Every retained COM provider keeps only a private serial plus shared bridge state. Property reads, navigation, pattern acquisition and action enqueue resolve that serial against the current native snapshot; a retired serial returns element/provider unavailable rather than reviving stale semantic state.
- Native action callbacks carry the provider serial plus exact document generation and geometry revision captured from the current native snapshot. `rarog-platform-windows` revalidates all three dimensions before creating a portable request, and `rarog-engine` independently revalidates the request against the current Rarog snapshot before delegating to ADR-0136. Platform/native objects never acquire action authority.
- Event publication is admitted only when provider serial, document generation and geometry revision exactly match the current native snapshot. Focus becomes platform-current only after the committed `FocusChanged` UI Automation event succeeds. Name/state changes use `UiaRaiseAutomationPropertyChangedEvent` only when exact old/new values are available; the bridge emits no fabricated generic property-change event when they are not. Structure, layout, invoke and focus notifications use their corresponding narrow UI Automation event APIs.
- Failed native event delivery cannot roll back or mutate Rarog DOM/layout/accessibility state. Wrapper retry state is explicitly bounded and may conservatively coalesce stale/detailed pressure to one current-root `TreeChanged`; unsupported value-pattern publication fails with the fixed unsupported classification rather than approximation.
- The desktop shell owns the real winit window and extracts its HWND. `WindowsPlatformHost` advertises accessibility only after the HWND-bound provider bridge is constructed successfully. The native boundary revalidates the window with `IsWindow`; window loss becomes a fixed native failure.
- Native/platform failures map to fixed Rarog classifications. No HRESULT, HWND, COM pointer, UIA runtime ID or private serial enters core Web semantics.

## Consequences

- Rarog DOM plus committed accessibility snapshot generations remain authoritative; the Windows provider tree is disposable derived state.
- Retained stale COM providers and delayed callbacks fail closed against current shared snapshot state.
- Wrapper replacement is atomic with respect to its correlation maps: a failed native replacement leaves the prior wrapper correlation state intact, while engine-side publication policy may subsequently clear derived platform state rather than expose it as current.
- Focus state cannot advance merely because an event was attempted; it advances only after successful committed UI Automation focus publication.
- Linux portability remains possible because all COM/UIA/raw-handle operations remain below target-specific boundaries and portable contracts import no Windows types.

## Non-goals

This decision establishes only the minimal HWND-bound UI Automation provider foundation required by the fourth R5 Accessibility slice. It does not claim complete UI Automation pattern/property/event coverage, MSAA compatibility, broad ARIA/Accessible Name/WebIDL completeness, assistive-technology compatibility certification, WPT/WebDriver/BiDi/real-Web qualification, signed compatibility profiles, or any R6 work. Those require separate later evidence.
