from pathlib import Path

layout = Path("crates/rarog-layout/src/lib.rs")
s = layout.read_text()
if "pub fn refresh_text_node(" not in s:
    marker = "pub fn fragment_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Option<&Fragment> {\n"
    addition = '''pub fn refresh_text_node(
    tree: &mut LayoutTree,
    document: &Document,
    dom_node: NodeId,
) -> bool {
    let Some(NodeKind::Text(text)) = document.node(dom_node).map(|node| &node.kind) else {
        return false;
    };
    refresh_text_node_recursive(&mut tree.root, dom_node, text)
}

fn refresh_text_node_recursive(node: &mut LayoutNode, dom_node: NodeId, text: &str) -> bool {
    if node.dom_node == Some(dom_node) {
        if !matches!(node.kind, LayoutNodeKind::Text(_)) {
            return false;
        }
        node.kind = LayoutNodeKind::Text(TextRun::new(text.to_owned()));
        node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
        return true;
    }

    let changed = node
        .children
        .iter_mut()
        .any(|child| refresh_text_node_recursive(child, dom_node, text));
    if changed {
        node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
    }
    changed
}

'''
    if marker not in s:
        raise SystemExit("layout marker missing")
    s = s.replace(marker, addition + marker, 1)
    layout.write_text(s)

engine = Path("crates/rarog-engine/src/lib.rs")
s = engine.read_text()
if "refresh_text_node," not in s:
    old = "    layout_document_with_styles, relayout_fragment_flow, relayout_fragment_subtree, relayout_tree,\n"
    new = "    layout_document_with_styles, refresh_text_node, relayout_fragment_flow,\n    relayout_fragment_subtree, relayout_tree,\n"
    if old not in s:
        raise SystemExit("import marker missing")
    s = s.replace(old, new, 1)

if "let mut text_relayout_nodes = BTreeSet::new();" not in s:
    marker = "        let mut requires_full_rebuild = mutation_history_lost;\n"
    if marker not in s:
        raise SystemExit("state marker missing")
    s = s.replace(marker, "        let mut text_relayout_nodes = BTreeSet::new();\n" + marker, 1)

if "text_relayout_nodes.insert(*node);" not in s:
    old = (
        "                MutationKind::CharacterData { node } => {\n"
        "                    requires_full_rebuild = true;\n"
        "                    stylesheet_sources_changed |=\n"
        "                        node_is_within_style_element(&self.document, *node);\n"
        "                }\n"
    )
    new = (
        "                MutationKind::CharacterData { node } => {\n"
        "                    if node_is_within_style_element(&self.document, *node) {\n"
        "                        requires_full_rebuild = true;\n"
        "                        stylesheet_sources_changed = true;\n"
        "                    } else {\n"
        "                        text_relayout_nodes.insert(*node);\n"
        "                    }\n"
        "                }\n"
    )
    if old not in s:
        raise SystemExit("character-data marker missing")
    s = s.replace(old, new, 1)

if "!text_relayout_nodes.is_empty() && !style_candidates.is_empty()" not in s:
    marker = "        let new_styles = if stylesheet_sources_changed {\n"
    guard = (
        "        if !text_relayout_nodes.is_empty() && !style_candidates.is_empty() {\n"
        "            requires_full_rebuild = true;\n"
        "        }\n\n"
    )
    if marker not in s:
        raise SystemExit("style guard marker missing")
    s = s.replace(marker, guard + marker, 1)

if "for node in &text_relayout_nodes" not in s:
    marker = "        let mode;\n        let patched_nodes;\n"
    addition = (
        "        if !requires_full_rebuild {\n"
        "            for node in &text_relayout_nodes {\n"
        "                if !refresh_text_node(&mut self.layout.tree, &self.document, *node) {\n"
        "                    requires_full_rebuild = true;\n"
        "                    break;\n"
        "                }\n"
        "                geometry_changed = true;\n"
        "                subtree_relayout_safe = false;\n"
        "                flow_relayout_nodes.insert(*node);\n"
        "            }\n"
        "        }\n\n"
    )
    if marker not in s:
        raise SystemExit("mode marker missing")
    s = s.replace(marker, addition + marker, 1)

old = "        } else if style_updates.is_empty() {\n            self.styles = new_styles;\n"
if old in s:
    s = s.replace(
        old,
        "        } else if style_updates.is_empty() && text_relayout_nodes.is_empty() {\n            self.styles = new_styles;\n",
        1,
    )

old = (
    "        } else if geometry_changed {\n"
    "            let previous_display_list = self.display_list.clone();\n"
    "            patched_nodes = style_updates.len();\n"
)
if old in s:
    s = s.replace(
        old,
        "        } else if geometry_changed {\n"
        "            let previous_display_list = self.display_list.clone();\n"
        "            patched_nodes = style_updates.len() + text_relayout_nodes.len();\n",
        1,
    )

if "fn character_data_change_reflows_existing_text_without_full_rebuild()" not in s:
    marker = "    #[test]\n    fn structural_change_still_falls_back_to_full_rebuild() {\n"
    tests = r'''    #[test]
    fn character_data_change_reflows_existing_text_without_full_rebuild() {
        let source = "<div id=\"before\" style=\"height:5px;background:#eeeeee\"></div><div id=\"target\" style=\"width:48px;background:#112233\">one</div><div id=\"after\" style=\"height:10px;background:#445566\"></div>";
        let expected_source = "<div id=\"before\" style=\"height:5px;background:#eeeeee\"></div><div id=\"target\" style=\"width:48px;background:#112233\">one two three four</div><div id=\"after\" style=\"height:10px;background:#445566\"></div>";
        let mut session = session(source, deterministic_options());
        let before = element_with_id(session.document(), "before");
        let target = element_with_id(session.document(), "target");
        let text = *session
            .document()
            .children(target)
            .and_then(|children| children.first())
            .expect("target contains a text node");
        let before_fragment_id = fragment_for_dom(&session.layout().fragments, before)
            .expect("before fragment exists")
            .id;
        let layout_node_count = session.layout().tree.node_count();

        session
            .document_mut()
            .set_text(text, "one two three four")
            .unwrap();
        let report = session.update().expect("incremental text update succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert_eq!(report.patched_nodes, 1);
        assert_eq!(session.layout().tree.node_count(), layout_node_count);
        assert_eq!(
            fragment_for_dom(&session.layout().fragments, before)
                .expect("unaffected prefix fragment remains")
                .id,
            before_fragment_id
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
        assert!(!session.damage().rects.is_empty());
    }

    #[test]
    fn style_element_character_data_still_requires_full_rebuild() {
        let mut session = session(
            "<style>#target { background:#112233; }</style><div id=\"target\" style=\"height:20px\"></div>",
            deterministic_options(),
        );
        let mut stack = vec![session.document().root()];
        let mut style_text = None;
        while let Some(node) = stack.pop() {
            if session
                .document()
                .node(node)
                .is_some_and(|node| matches!(node.kind, NodeKind::Text(_)))
                && node_is_within_style_element(session.document(), node)
            {
                style_text = Some(node);
                break;
            }
            stack.extend_from_slice(session.document().children(node).unwrap_or(&[]));
        }
        let style_text = style_text.expect("fixture contains style text");

        session
            .document_mut()
            .set_text(style_text, "#target { background:#445566; }")
            .unwrap();
        let report = session.update().expect("stylesheet text update succeeds");

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
    }

'''
    if marker not in s:
        raise SystemExit("test marker missing")
    s = s.replace(marker, tests + marker, 1)

engine.write_text(s)

inline_test = Path("crates/rarog-engine/tests/r1_inline_fragmentation.rs")
s = inline_test.read_text()
s = s.replace(
    "fn text_growth_rebuilds_inline_fragments_to_match_fresh_render() {",
    "fn text_growth_reflows_inline_fragments_to_match_fresh_render() {",
    1,
)
s = s.replace(
    "assert_eq!(report.mode, IncrementalMode::FullRebuild);\n    assert_eq!(\n        session.framebuffer().stable_hash64(),\n        fresh.framebuffer.stable_hash64()\n    );",
    "assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n    assert_eq!(\n        session.framebuffer().stable_hash64(),\n        fresh.framebuffer.stable_hash64()\n    );",
    1,
)
inline_test.write_text(s)
