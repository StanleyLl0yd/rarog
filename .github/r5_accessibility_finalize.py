from pathlib import Path

BASE = "ae7033b524dbbe71f1294b44b8a14686f5540317"

source = Path("crates/rarog-accessibility/src/lib.rs")
text = source.read_text()


def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, got {count}")
    text = text.replace(old, new, 1)


old_limit = """            let next_nodes =
                drafts
                    .len()
                    .checked_add(1)
                    .ok_or(AccessibilityError::NodeLimitExceeded {
                        nodes: usize::MAX,
                        limit: self.limits.max_nodes,
                    })?;
            if next_nodes > self.limits.max_nodes {
                return Err(AccessibilityError::NodeLimitExceeded {
                    nodes: next_nodes,
                    limit: self.limits.max_nodes,
                });
            }
"""
if text.count(old_limit) != 1:
    raise SystemExit(f"node-limit block: expected one match, got {text.count(old_limit)}")
text = text.replace(old_limit, "", 1)
marker = "            let role = role_for_node(&node.kind);\n"
if text.count(marker) != 1:
    raise SystemExit("role marker changed")
text = text.replace(marker, old_limit + marker, 1)

replace_once(
    """                role,
                self.limits.max_name_bytes_per_node,
                self.limits.max_dom_nodes_scanned,
""",
    """                role,
                &bounds,
                self.limits.max_name_bytes_per_node,
                self.limits.max_dom_nodes_scanned,
""",
    "name_for_node call",
)
replace_once(
    """    role: AccessibilityRole,
    limit: usize,
    dom_node_limit: usize,
""",
    """    role: AccessibilityRole,
    rendered_bounds: &BTreeMap<NodeId, Rect>,
    limit: usize,
    dom_node_limit: usize,
""",
    "name_for_node signature",
)
replace_once(
    "                    descendant_text_name(document, source, limit, dom_node_limit)\n",
    """                    descendant_text_name(
                        document,
                        source,
                        rendered_bounds,
                        limit,
                        dom_node_limit,
                    )
""",
    "descendant name call",
)
replace_once(
    """    source: NodeId,
    limit: usize,
    dom_node_limit: usize,
) -> Result<String, AccessibilityError> {
""",
    """    source: NodeId,
    rendered_bounds: &BTreeMap<NodeId, Rect>,
    limit: usize,
    dom_node_limit: usize,
) -> Result<String, AccessibilityError> {
""",
    "descendant name signature",
)
replace_once(
    "            NodeKind::Text(text) => accumulator.push_text(text)?,\n",
    """            NodeKind::Text(text) => {
                if rendered_bounds.contains_key(&current) {
                    accumulator.push_text(text)?;
                }
            }
""",
    "rendered text filter",
)

test = r'''

    #[test]
    fn descendant_name_ignores_text_without_rendered_source_geometry() {
        fn clear_source(fragment: &mut Fragment, source: NodeId) {
            if fragment.dom_node == Some(source) {
                fragment.dom_node = None;
            }
            for child in &mut fragment.children {
                clear_source(child, source);
            }
        }

        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        document
            .append_new(button, NodeKind::Text("Visible".into()))
            .unwrap();
        let hidden_text = document
            .append_new(button, NodeKind::Text("Hidden".into()))
            .unwrap();
        let mut layout = layout_document(&document, viewport());
        clear_source(&mut layout.fragments.root, hidden_text);

        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        assert_eq!(tree.node_for_source(button).unwrap().name(), "Visible");
        assert!(tree.node_for_source(hidden_text).is_none());
    }
'''
if "fn descendant_name_ignores_text_without_rendered_source_geometry()" in text:
    raise SystemExit("rendered-name test already present")
closing = text.rfind("\n}")
if closing < 0:
    raise SystemExit("test module closing brace missing")
text = text[:closing] + test + text[closing:]
source.write_text(text)

adr = Path("docs/adr/0135-accessibility-tree-foundation.md")
adr_text = adr.read_text()
replacements = [
    (
        "- The bootstrap native-HTML role vocabulary is deliberately bounded: root web area, generic container, static text, button, link, heading, text field, checkbox, radio button, image, list and list item. This is an architectural foundation, not HTML/ARIA accessibility-role completeness.",
        "- The bootstrap native-HTML role vocabulary is deliberately bounded: root web area, generic container, static text, button, link, heading, text field, checkbox, radio button, image, list and list item. Native element-role mapping is applied only to the HTML namespace; unsupported input types remain generic rather than receiving approximate text-field semantics. This is an architectural foundation, not HTML/ARIA accessibility-role completeness.",
    ),
    (
        "- The bootstrap accessible-name rules are deliberately bounded to selected Rarog-owned DOM inputs: normalized text, bounded `aria-label`, image `alt`, button-like input `value`, and normalized descendant text for selected native roles. Name strings are accumulated under a per-node byte limit rather than constructed without bounds and checked afterward.",
        "- The bootstrap accessible-name rules are deliberately bounded to selected Rarog-owned DOM inputs: normalized text, bounded `aria-label`, image `alt`, button-like input `value`, and normalized rendered-descendant text for selected native roles. Descendant text without current FragmentTree source geometry does not contribute. Name strings are accumulated under a per-node byte limit rather than constructed without bounds and checked afterward.",
    ),
    (
        "- `AccessibilityLimits` explicitly bounds exposed nodes, retained identities, FragmentTree traversal, bytes per accessible name and aggregate accessible-name bytes. Count/byte arithmetic is checked and invalid/non-finite geometry fails closed.",
        "- `AccessibilityLimits` explicitly bounds exposed nodes, retained identities, connected DOM nodes scanned, FragmentTree traversal, bytes per accessible name and aggregate accessible-name bytes. DOM/fragment worklists are bounded before enqueue, count/byte arithmetic is checked, node capacity is rejected before accessible-name work, and invalid/non-finite geometry fails closed.",
    ),
]
for old, new in replacements:
    count = adr_text.count(old)
    if count != 1:
        raise SystemExit(f"ADR-0135 wording changed: expected one match, got {count}")
    adr_text = adr_text.replace(old, new, 1)
adr.write_text(adr_text)

architecture = Path("docs/ARCHITECTURE.md")
arch = architecture.read_text()
marker = "## Platform host boundary\n"
if marker not in arch:
    raise SystemExit("Architecture platform marker missing")
if "## R5 platform-neutral accessibility tree foundation" in arch:
    raise SystemExit("Architecture accessibility section already present")
section = """## R5 platform-neutral accessibility tree foundation

R5 introduces `rarog-accessibility` as a portable derived-state boundary. The DOM remains semantic authority and the Fragment Tree contributes geometry only; accessibility IDs, Fragment IDs, LayoutNode IDs and later platform/provider IDs are correlation identities, not interchangeable authority. `AccessibilityTreeState` is owned for one document lifecycle and allocates a fresh Rarog scope plus monotonic non-zero `AccessibilityNodeId` values. A new document lifecycle must receive a new state.

Each immutable `AccessibilityTree` snapshot records the exact source DOM generation and contains only current exposed nodes. Non-root sources require current FragmentTree source geometry; omitted intermediates are flattened to the nearest exposed DOM ancestor without changing DOM order. Multiple fragment border boxes for one DOM source are validated and unioned into one portable `Rect`. Fragment/layout allocation identity never escapes as accessibility authority.

The first Accessibility slice deliberately implements only a narrow HTML-native semantic foundation: root web area, generic/static text, button, link, heading, selected text inputs, checkbox/radio, image and list/list-item roles; normalized bounded names; and fixed disabled/checked/expanded/focusable state. Native element-role mapping is HTML-namespace-only, unsupported input types remain generic, and descendant text contributes to selected names only when that text source has current rendered fragment geometry. This is not HTML/ARIA role or Accessible Name algorithm completeness.

Accessibility resource use is explicit and fail-closed. `AccessibilityLimits` bounds exposed nodes, retained identities, connected DOM nodes scanned, fragments traversed, name bytes per node and aggregate retained name bytes. DOM/fragment worklists are bounded before enqueue, arithmetic is checked, names are built under their byte limits, and malformed/non-finite geometry rejects the snapshot before retained identity state is committed. Stable identities survive rebuild and detach/re-attach within one document lifecycle, while stale IDs do not resolve in snapshots where their source is absent.

Actions/events, mutation/render-to-accessibility invalidation and the Windows accessibility bridge remain later R5 slices. This foundation contains no Windows UIA/MSAA/COM/native accessibility objects and makes no WPT, assistive-technology compatibility or R6 qualification claim. See ADR-0135.

"""
arch = arch.replace(marker, section + marker, 1)
architecture.write_text(arch)
