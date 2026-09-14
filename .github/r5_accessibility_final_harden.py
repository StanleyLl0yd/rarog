from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one match, found {count}")
    target.write_text(text.replace(old, new, 1))


# Portable projection validation must prove one exact rooted tree, not merely valid references.
replace_once(
    "crates/rarog-platform/src/accessibility.rs",
    "use std::collections::BTreeSet;",
    "use std::collections::{BTreeMap, BTreeSet};",
)
replace_once(
    "crates/rarog-platform/src/accessibility.rs",
    '''        let scope = root.scope();
        let mut ids = BTreeSet::new();
        for node in &nodes {
            if node.id().scope() != scope || !ids.insert(node.id()) {
                return Err(invalid_snapshot());
            }
            if node.bounds().is_some_and(|bounds| !bounds.is_valid()) {
                return Err(invalid_snapshot());
            }
        }
        if !ids.contains(&root) {
            return Err(invalid_snapshot());
        }
        for node in &nodes {
            if node.parent().is_some_and(|parent| !ids.contains(&parent))
                || node.children().iter().any(|child| !ids.contains(child))
            {
                return Err(invalid_snapshot());
            }
        }
''',
    '''        let scope = root.scope();
        let mut indices = BTreeMap::new();
        for (index, node) in nodes.iter().enumerate() {
            if node.id().scope() != scope || indices.insert(node.id(), index).is_some() {
                return Err(invalid_snapshot());
            }
            if node.bounds().is_some_and(|bounds| !bounds.is_valid()) {
                return Err(invalid_snapshot());
            }
        }
        let Some(&root_index) = indices.get(&root) else {
            return Err(invalid_snapshot());
        };
        if nodes[root_index].parent().is_some() {
            return Err(invalid_snapshot());
        }

        for node in &nodes {
            let mut children = BTreeSet::new();
            for child in node.children() {
                if !children.insert(*child) {
                    return Err(invalid_snapshot());
                }
                let Some(&child_index) = indices.get(child) else {
                    return Err(invalid_snapshot());
                };
                if nodes[child_index].parent() != Some(node.id()) {
                    return Err(invalid_snapshot());
                }
            }

            match node.parent() {
                Some(parent) => {
                    if node.id() == root {
                        return Err(invalid_snapshot());
                    }
                    let Some(&parent_index) = indices.get(&parent) else {
                        return Err(invalid_snapshot());
                    };
                    if nodes[parent_index]
                        .children()
                        .iter()
                        .filter(|&&child| child == node.id())
                        .count()
                        != 1
                    {
                        return Err(invalid_snapshot());
                    }
                }
                None if node.id() != root => return Err(invalid_snapshot()),
                None => {}
            }
        }

        let mut visited = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(current) = pending.pop() {
            if !visited.insert(current) {
                return Err(invalid_snapshot());
            }
            let Some(&index) = indices.get(&current) else {
                return Err(invalid_snapshot());
            };
            pending.extend(nodes[index].children().iter().rev().copied());
        }
        if visited.len() != nodes.len() {
            return Err(invalid_snapshot());
        }
''',
)
replace_once(
    "crates/rarog-platform/src/accessibility.rs",
    '''    #[test]
    fn snapshot_accepts_bounded_tree_projection_shape() {
''',
    '''    #[test]
    fn snapshot_rejects_root_parent_duplicate_children_and_parent_mismatch() {
        let root = id(1);
        let child = id(2);
        let root_with_parent = vec![
            PlatformAccessibilityNode::new(
                root,
                PlatformAccessibilityRole::RootWebArea,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                Some(child),
                Vec::new(),
            ),
            node(2, Some(1)),
        ];
        assert_eq!(
            PlatformAccessibilitySnapshot::try_new(1, 1, root, root_with_parent)
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::InvalidSnapshot
        );

        let duplicate_child = vec![
            PlatformAccessibilityNode::new(
                root,
                PlatformAccessibilityRole::RootWebArea,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                None,
                vec![child, child],
            ),
            node(2, Some(1)),
        ];
        assert_eq!(
            PlatformAccessibilitySnapshot::try_new(1, 1, root, duplicate_child)
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::InvalidSnapshot
        );

        let parent_mismatch = vec![
            PlatformAccessibilityNode::new(
                root,
                PlatformAccessibilityRole::RootWebArea,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                None,
                vec![child],
            ),
            node(2, None),
        ];
        assert_eq!(
            PlatformAccessibilitySnapshot::try_new(1, 1, root, parent_mismatch)
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::InvalidSnapshot
        );
    }

    #[test]
    fn snapshot_rejects_disconnected_cycle() {
        let root = id(1);
        let second = id(2);
        let third = id(3);
        let nodes = vec![
            PlatformAccessibilityNode::new(
                root,
                PlatformAccessibilityRole::RootWebArea,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                None,
                Vec::new(),
            ),
            PlatformAccessibilityNode::new(
                second,
                PlatformAccessibilityRole::GenericContainer,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                Some(third),
                vec![third],
            ),
            PlatformAccessibilityNode::new(
                third,
                PlatformAccessibilityRole::GenericContainer,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                Some(second),
                vec![second],
            ),
        ];
        assert_eq!(
            PlatformAccessibilitySnapshot::try_new(1, 1, root, nodes)
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::InvalidSnapshot
        );
    }

    #[test]
    fn snapshot_accepts_bounded_tree_projection_shape() {
''',
)

# A handle can become invalid after attachment. Revalidate at every native publication.
replace_once(
    "crates/rarog-platform-windows-native/src/accessibility.rs",
    '''    ) -> Result<(), WindowsAccessibilityNativeError> {
        publish_win_event(self.hwnd, provider_serial, kind)
    }
''',
    '''    ) -> Result<(), WindowsAccessibilityNativeError> {
        validate_window(self.hwnd)?;
        publish_win_event(self.hwnd, provider_serial, kind)
    }
''',
)
replace_once(
    "crates/rarog-platform-windows-native/src/accessibility.rs",
    '''    #[test]
    fn construction_fails_closed_without_a_supported_live_window() {
''',
    '''    #[test]
    fn publication_revalidates_the_bound_window() {
        let fake = NonZeroIsize::new(1).expect("non-zero test handle");
        let mut bridge = WindowsAccessibilityNativeBridge { hwnd: fake };
        let error = bridge
            .publish_event(1, WindowsAccessibilityNativeEventKind::TreeChanged)
            .unwrap_err();
        assert_eq!(
            error.kind(),
            if cfg!(target_os = "windows") {
                WindowsAccessibilityNativeErrorKind::InvalidWindow
            } else {
                WindowsAccessibilityNativeErrorKind::UnsupportedTarget
            }
        );
    }

    #[test]
    fn construction_fails_closed_without_a_supported_live_window() {
''',
)

# Reconcile the architecture narrative with the completed Windows bridge slice.
replace_once(
    "docs/ARCHITECTURE.md",
    "Mutation/render-to-accessibility invalidation and the Windows accessibility bridge remain later R5 slices. This foundation contains no Windows UIA/MSAA/COM/native accessibility objects and makes no WPT, assistive-technology compatibility or R6 qualification claim. See ADR-0135.",
    "Mutation/render-to-accessibility invalidation and Windows accessibility bridging are layered by the later R5 slices below. This foundation itself contains no Windows UIA/MSAA/COM/native accessibility objects and makes no WPT, assistive-technology compatibility or R6 qualification claim. See ADR-0135.",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "A successful DOM-backed action advances the ordinary DOM generation, so the old accessibility snapshot becomes stale until a later rebuild; automatic invalidation remains the next R5 Accessibility slice.",
    "A successful DOM-backed action advances the ordinary DOM generation, so the old accessibility snapshot becomes stale until a later rebuild; automatic invalidation is owned by the following R5 Accessibility slice rather than this action boundary.",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "This boundary contains no Windows UIA/MSAA/COM/provider object and no platform accessibility identifier becomes Web authority. The portable semantic event queue is the handoff consumed by the next Windows accessibility bridge slice. No broad ARIA completeness, assistive-technology compatibility, WPT/WebDriver/BiDi/real-Web qualification or R6 work is claimed. See ADR-0137.",
    "This boundary contains no Windows UIA/MSAA/COM/provider object and no platform accessibility identifier becomes Web authority. Its committed snapshot and portable semantic event queue are consumed by the Windows accessibility bridge boundary below. No broad ARIA completeness, assistive-technology compatibility, WPT/WebDriver/BiDi/real-Web qualification or R6 work is claimed. See ADR-0137.",
)
section = '''## R5 Windows accessibility bridge boundary

The fourth Accessibility slice adds a Windows host translation layer without changing Web authority. `rarog-platform` defines an immutable platform-neutral snapshot/event/action projection containing only Rarog correlation identities, fixed semantic roles/state, bounded names/geometry, exact document generation and geometry revision. Snapshot validation requires one rooted, reciprocal, duplicate-free and fully reachable tree. Windows HWND, WinEvent values, provider serials and native error values never enter this portable contract.

`rarog-platform-windows` owns bounded private provider correlations and queues. Provider serials are monotonic process-local correlation values; they are neither `AccessibilityNodeId` values nor OS authority and are never reused. Snapshot replacement preserves correlations only for retained current Rarog nodes, retires removed providers deterministically and drops queued work whose target is no longer current. Native action callbacks must match the exact live provider correlation, lifecycle scope, document generation and geometry revision before a fixed platform action request can be queued. The engine then revalidates the request against the current Rarog snapshot and delegates to the existing ADR-0136 action path before any DOM mutation.

The desktop shell owns the real winit window and is the only layer that extracts its Win32 HWND. `WindowsPlatformHost` starts with accessibility capability disabled; accessibility is attached only after that HWND exists. `rarog-platform-windows-native` validates the handle with `IsWindow` at attachment and again before every native publication, then emits the narrow WinEvent notification vocabulary using that HWND. A destroyed or invalid window therefore fails closed through the fixed native error classification and bounded retry/coalescing path rather than being treated as successful publication. No HWND or other raw Windows handle is exported into engine/core Web crates.

Snapshot publication is ordered before semantic events. A failed platform snapshot replacement causes the engine to retire the old platform snapshot rather than expose stale derived state as current. Native publication failure cannot roll back or mutate DOM/layout/accessibility authority; pending notifications remain bounded and may conservatively coalesce to one root `TreeChanged`. Platform action and notification queues have explicit non-zero capacity, and provider-map capacity is checked before replacement commits.

This is the first HWND-bound Windows accessibility bridge, not a claim of complete UI Automation or MSAA provider-object/pattern support. It does not implement broad native provider patterns, broad ARIA/WebIDL accessibility surfaces, assistive-technology compatibility certification, WPT/WebDriver/BiDi/real-Web qualification or any R6 work. Those claims require their own later evidence and must not be inferred from WinEvent publication or the bounded native callback seam. See ADR-0138.

'''
replace_once(
    "docs/ARCHITECTURE.md",
    "## Platform host boundary\n",
    section + "## Platform host boundary\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "`WindowsPlatformHost::try_new` still succeeds only on a Windows compilation target, while both Windows crates remain buildable on Linux for portability CI. Accessibility remains R5 work. See ADR-0030, ADR-0108, ADR-0110 and ADR-0111.",
    "`WindowsPlatformHost::try_new` still succeeds only on a Windows compilation target, while both Windows crates remain buildable on Linux for portability CI. R5 accessibility follows the same containment rule: the platform host exposes only the portable service contract, while HWND validation and WinEvent publication stay in the Windows/native layers described above. See ADR-0030, ADR-0108, ADR-0110, ADR-0111 and ADR-0138.",
)

Path("docs/adr/0138-windows-accessibility-bridge.md").write_text('''# ADR-0138: HWND-bound Windows accessibility bridge without platform authority

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
''')

print("final accessibility bridge hardening staged")
