from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
UIA = ROOT / "crates/rarog-platform-windows-native/src/accessibility_uia.rs"
LIB = ROOT / "crates/rarog-platform-windows-native/src/lib.rs"
ADR = ROOT / "docs/adr/0138-windows-accessibility-bridge.md"
ARCH = ROOT / "docs/ARCHITECTURE.md"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def patch_uia_semantics() -> None:
    text = UIA.read_text(encoding="utf-8")

    text = replace_once(
        text,
        "            UIA_AutomationFocusChangedEventId, UIA_AutomationPropertyChangedEventId,\n",
        "            UIA_AutomationFocusChangedEventId,\n",
        "generic property-event import",
    )
    text = replace_once(
        text,
        "                _ => UiaRaiseAutomationEvent(&provider, UIA_AutomationPropertyChangedEventId),\n",
        "                _ => Ok(()),\n",
        "name-change generic fallback",
    )
    text = replace_once(
        text,
        "    let (Some(previous), Some(current)) = (previous, current) else {\n        return unsafe { UiaRaiseAutomationEvent(provider, UIA_AutomationPropertyChangedEventId) };\n    };\n    let mut raised = false;\n",
        "    let (Some(previous), Some(current)) = (previous, current) else {\n        return Ok(());\n    };\n",
        "state-change missing-value fallback",
    )
    text = replace_once(
        text,
        "    if !raised {\n        unsafe { UiaRaiseAutomationEvent(provider, UIA_AutomationPropertyChangedEventId) }?;\n    }\n    Ok(())\n",
        "    Ok(())\n",
        "state-change no-exact-value fallback",
    )

    raised_count = text.count("raised = true;")
    if raised_count != 4:
        raise RuntimeError(f"state-change bookkeeping: expected four assignments, found {raised_count}")
    text = text.replace("        raised = true;\n", "")
    text = text.replace("            raised = true;\n", "")
    if "raised = true;" in text:
        raise RuntimeError("state-change bookkeeping remains after no-op fallback removal")

    old_focus = '''            let current_node = current.nodes.get(&provider_serial).cloned();
            let previous_node = state
                .previous
                .as_ref()
                .and_then(|previous| previous.nodes.get(&provider_serial))
                .cloned();
            if kind == WindowsAccessibilityNativeEventKind::FocusChanged {
                state.focused_provider = Some(provider_serial);
            }
            (current_node, previous_node)
        };
        let provider = simple_provider_from_arc(&self.shared, provider_serial)?;
        raise_event(provider, kind, event_data.0.as_ref(), event_data.1.as_ref())
'''
    new_focus = '''            let current_node = current.nodes.get(&provider_serial).cloned();
            let previous_node = state
                .previous
                .as_ref()
                .and_then(|previous| previous.nodes.get(&provider_serial))
                .cloned();
            (current_node, previous_node)
        };
        let provider = simple_provider_from_arc(&self.shared, provider_serial)?;
        raise_event(provider, kind, event_data.0.as_ref(), event_data.1.as_ref())?;
        if kind == WindowsAccessibilityNativeEventKind::FocusChanged {
            let mut state = self.shared.lock()?;
            let current = state.current.as_ref().ok_or_else(|| {
                native_error(WindowsAccessibilityNativeErrorKind::ProviderUnavailable)
            })?;
            if current.document_generation != document_generation
                || current.geometry_revision != geometry_revision
                || !current.nodes.contains_key(&provider_serial)
            {
                return Err(native_error(
                    WindowsAccessibilityNativeErrorKind::ProviderUnavailable,
                ));
            }
            state.focused_provider = Some(provider_serial);
        }
        Ok(())
'''
    text = replace_once(text, old_focus, new_focus, "focus commit ordering")

    if "UIA_AutomationPropertyChangedEventId" in text:
        raise RuntimeError("generic UIA property-changed event fallback still present")
    if "UiaRaiseAutomationPropertyChangedEvent" not in text:
        raise RuntimeError("exact UIA property-change API unexpectedly absent")

    UIA.write_text(text, encoding="utf-8")


ADR_TEXT = '''# ADR-0138: HWND-bound Windows UI Automation provider bridge without platform authority

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
'''


ARCH_SECTION = '''## R5 Windows accessibility bridge boundary

The fourth Accessibility slice adds a real Windows UI Automation provider translation layer without changing Web authority. `rarog-platform` defines an immutable platform-neutral snapshot/event/action projection containing only Rarog correlation identities, fixed semantic roles/state, bounded names/geometry, exact document generation and geometry revision. Snapshot validation requires one rooted, reciprocal, duplicate-free and fully reachable tree. Windows HWND, COM objects, UIA runtime IDs, private provider serials and native error values never enter this portable contract.

`rarog-platform-windows` owns bounded private provider correlations and retry/action state. Provider serials are non-zero monotonic process-local correlation values bounded by `i32::MAX`; they are neither `AccessibilityNodeId` values nor OS authority and are never reused. Snapshot replacement preserves correlations only for retained current Rarog nodes and commits wrapper maps only after native replacement succeeds. Native action callbacks must match the exact live provider correlation, lifecycle scope, document generation and geometry revision before a fixed platform action request can be returned. The engine then independently revalidates that request against the current Rarog snapshot and delegates to the ADR-0136 action path before any DOM mutation.

The desktop shell owns the real winit window and is the only layer that extracts its Win32 HWND. `WindowsPlatformHost` starts with accessibility capability disabled and attaches accessibility only after that HWND exists. `rarog-platform-windows-native` validates the handle, subclasses the HWND for `WM_GETOBJECT`, returns the root through `UiaReturnRawElementProvider`, and implements the minimal `IRawElementProviderSimple`/fragment/root provider foundation plus Invoke, Toggle and ExpandCollapse patterns. Retained COM providers resolve their private serial against current shared snapshot state on every operation, so retired providers fail closed instead of becoming semantic authority.

Native event publication requires the exact current provider serial, document generation and geometry revision. Name/state property changes call `UiaRaiseAutomationPropertyChangedEvent` only with exact old/new values; missing value pairs produce no fabricated generic property-change event. Focus becomes platform-current only after the committed UIA focus event succeeds. Structure, layout, invoke and focus notifications use the corresponding narrow UI Automation event APIs. Native delivery failure cannot roll back DOM/layout/accessibility authority; retry pressure remains bounded and may conservatively coalesce to one current-root `TreeChanged`.

This is a minimal HWND-bound UI Automation provider foundation, not a complete accessibility implementation. It does not claim complete UIA property/pattern/event coverage, MSAA compatibility, broad ARIA/WebIDL accessibility surfaces, assistive-technology compatibility certification, WPT/WebDriver/BiDi/real-Web qualification or any R6 work. See ADR-0138.

'''


def rewrite_docs() -> None:
    ADR.write_text(ADR_TEXT, encoding="utf-8")

    text = ARCH.read_text(encoding="utf-8")
    start = text.index("## R5 Windows accessibility bridge boundary\n")
    end = text.index("## Platform host boundary\n", start)
    text = text[:start] + ARCH_SECTION + text[end:]
    text = replace_once(
        text,
        "R5 accessibility follows the same containment rule: the platform host exposes only the portable service contract, while HWND validation and WinEvent publication stay in the Windows/native layers described above.",
        "R5 accessibility follows the same containment rule: the platform host exposes only the portable service contract, while HWND-bound UI Automation provider and event publication stays in the Windows/native layers described above.",
        "platform-host accessibility narrative",
    )
    ARCH.write_text(text, encoding="utf-8")


patch_uia_semantics()
rewrite_docs()

lib = LIB.read_text(encoding="utf-8")
if "JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE" not in lib:
    raise RuntimeError("R4 sandbox invariant lost: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE missing")
if "JOB_OBJECT_LIMIT_KILL_ON_CLOSE" in lib:
    raise RuntimeError("invalid shortened job-object constant remains")

uia = UIA.read_text(encoding="utf-8")
if "UIA_AutomationPropertyChangedEventId" in uia:
    raise RuntimeError("generic property-changed event identifier remains")
if "state.focused_provider = Some(provider_serial);" not in uia:
    raise RuntimeError("committed focus update unexpectedly absent")
