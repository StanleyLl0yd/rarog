from pathlib import Path

layout_path = Path("crates/rarog-layout/src/lib.rs")
layout = layout_path.read_text()
start = layout.find("pub fn refresh_layout_subtree(\n")
end = layout.find("fn max_layout_node_id(node: &LayoutNode) -> usize {", start)
if start < 0 or end < 0:
    if "pub fn refresh_layout_subtrees(" not in layout:
        raise SystemExit("layout subtree refresh block missing")
else:
    replacement = '''pub fn refresh_layout_subtree(
    tree: &mut LayoutTree,
    document: &Document,
    styles: &StyleSet,
    dom_node: NodeId,
) -> bool {
    refresh_layout_subtrees(tree, document, styles, &[dom_node])
}

pub fn refresh_layout_subtrees(
    tree: &mut LayoutTree,
    document: &Document,
    styles: &StyleSet,
    dom_nodes: &[NodeId],
) -> bool {
    if dom_nodes.is_empty() {
        return false;
    }

    let mut retained_ids = std::collections::BTreeMap::new();
    collect_layout_node_ids(&tree.root, &mut retained_ids);
    let mut next_id = max_layout_node_id(&tree.root).saturating_add(1);
    for dom_node in dom_nodes {
        if !refresh_layout_subtree_recursive(
            &mut tree.root,
            document,
            styles,
            *dom_node,
            None,
            &retained_ids,
            &mut next_id,
        ) {
            return false;
        }
    }
    true
}

fn refresh_layout_subtree_recursive(
    node: &mut LayoutNode,
    document: &Document,
    styles: &StyleSet,
    dom_node: NodeId,
    parent_style: Option<ComputedStyle>,
    retained_ids: &std::collections::BTreeMap<NodeId, LayoutNodeId>,
    next_id: &mut usize,
) -> bool {
    if node.dom_node == Some(dom_node) {
        let mut builder = LayoutTreeBuilder {
            next_id: *next_id,
            styles,
        };
        let Some(mut replacement) = builder.build_node(document, dom_node, parent_style) else {
            return false;
        };
        *next_id = builder.next_id;
        reuse_layout_node_ids(&mut replacement, retained_ids);
        *node = replacement;
        return true;
    }

    let style = node.style;
    for child in &mut node.children {
        if refresh_layout_subtree_recursive(
            child,
            document,
            styles,
            dom_node,
            Some(style),
            retained_ids,
            next_id,
        ) {
            node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
            return true;
        }
    }
    false
}

'''
    layout = layout[:start] + replacement + layout[end:]
layout_path.write_text(layout)

engine_path = Path("crates/rarog-engine/src/lib.rs")
s = engine_path.read_text()
s = s.replace(
    "layout_document_with_styles, refresh_layout_subtree,\n    refresh_text_node,",
    "layout_document_with_styles, refresh_layout_subtrees, refresh_text_node,",
    1,
)

old_reparent = '''                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed |=
                        subtree_contains_style_element(&self.document, *child)
                            || old_parent.is_some_and(|parent| {
                                node_is_within_style_element(&self.document, parent)
                            })
                            || new_parent.is_some_and(|parent| {
                                node_is_within_style_element(&self.document, parent)
                            });
                }
'''
new_reparent = '''                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    let stylesheet_source_changed =
                        subtree_contains_style_element(&self.document, *child)
                            || old_parent.is_some_and(|parent| {
                                node_is_within_style_element(&self.document, parent)
                            })
                            || new_parent.is_some_and(|parent| {
                                node_is_within_style_element(&self.document, parent)
                            });
                    stylesheet_sources_changed |= stylesheet_source_changed;
                    if stylesheet_source_changed {
                        requires_full_rebuild = true;
                    } else {
                        for parent in old_parent.iter().chain(new_parent.iter()) {
                            if self.document.is_connected(*parent) {
                                structural_relayout_nodes.insert(*parent);
                            }
                        }
                    }
                }
'''
if old_reparent in s:
    s = s.replace(old_reparent, new_reparent, 1)
elif "for parent in old_parent.iter().chain(new_parent.iter())" not in s:
    raise SystemExit("reparent mutation marker missing")

old_refresh = '''        if !requires_full_rebuild && !structural_relayout_nodes.is_empty() {
            for node in structural_relayout_nodes.iter().copied() {
                if !refresh_layout_subtree(&mut self.layout.tree, &self.document, &new_styles, node)
                {
                    requires_full_rebuild = true;
                    break;
                }
                geometry_changed = true;
                subtree_relayout_safe = false;
                flow_relayout_nodes.insert(node);
            }
            if !requires_full_rebuild {
                style_candidates.retain(|candidate| {
                    !structural_relayout_nodes
                        .iter()
                        .any(|root| node_is_within_dom_subtree(&self.document, *root, *candidate))
                });
            }
        }
'''
new_refresh = '''        if !requires_full_rebuild && !structural_relayout_nodes.is_empty() {
            structural_relayout_nodes =
                minimal_structural_roots(&self.document, &structural_relayout_nodes);
            let structural_roots = structural_relayout_nodes.iter().copied().collect::<Vec<_>>();
            if !refresh_layout_subtrees(
                &mut self.layout.tree,
                &self.document,
                &new_styles,
                &structural_roots,
            ) {
                requires_full_rebuild = true;
            } else {
                geometry_changed = true;
                subtree_relayout_safe = false;
                flow_relayout_nodes.extend(structural_roots);
                style_candidates.retain(|candidate| {
                    !structural_relayout_nodes
                        .iter()
                        .any(|root| node_is_within_dom_subtree(&self.document, *root, *candidate))
                });
            }
        }
'''
if old_refresh in s:
    s = s.replace(old_refresh, new_refresh, 1)
elif "minimal_structural_roots(&self.document" not in s:
    raise SystemExit("structural refresh marker missing")

helper_marker = "fn node_is_within_dom_subtree(document: &Document, root: NodeId, mut node: NodeId) -> bool {\n"
helper = '''fn minimal_structural_roots(
    document: &Document,
    roots: &BTreeSet<NodeId>,
) -> BTreeSet<NodeId> {
    roots
        .iter()
        .copied()
        .filter(|candidate| {
            !roots.iter().copied().any(|other| {
                other != *candidate && node_is_within_dom_subtree(document, other, *candidate)
            })
        })
        .collect()
}

'''
if "fn minimal_structural_roots(" not in s:
    if helper_marker not in s:
        raise SystemExit("minimal structural roots marker missing")
    s = s.replace(helper_marker, helper + helper_marker, 1)

start = s.find("    #[test]\n    fn reparent_still_falls_back_to_full_rebuild() {")
end = s.find("    #[test]\n    fn unrelated_attribute_change_does_not_rebuild_render_state() {", start)
if start < 0 or end < 0:
    if "fn reparent_reflows_both_retained_parents()" not in s:
        raise SystemExit("reparent unit-test marker missing")
else:
    tests = '''    #[test]
    fn reparent_reflows_both_retained_parents() {
        let source = r#"<style>#from > span:last-child { height:7px;background:#112233; } #to > span:last-child { height:12px;background:#445566; }</style><div id="from"><span id="child">R</span></div><div id="to"><span id="existing">E</span></div>"#;
        let expected_source = r#"<style>#from > span:last-child { height:7px;background:#112233; } #to > span:last-child { height:12px;background:#445566; }</style><div id="from"></div><div id="to"><span id="existing">E</span><span id="child">R</span></div>"#;
        let mut session = session(source, deterministic_options());
        let from = element_with_id(session.document(), "from");
        let to = element_with_id(session.document(), "to");
        let child = element_with_id(session.document(), "child");
        let existing = element_with_id(session.document(), "existing");
        let from_layout = layout_id_for_dom(&session.layout().tree.root, from).unwrap();
        let to_layout = layout_id_for_dom(&session.layout().tree.root, to).unwrap();
        let child_layout = layout_id_for_dom(&session.layout().tree.root, child).unwrap();
        let existing_layout = layout_id_for_dom(&session.layout().tree.root, existing).unwrap();

        session.document_mut().append_child(to, child).unwrap();
        let report = session.update().expect("reparent reflow succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert_eq!(report.patched_nodes, 2);
        assert!(report.retained_display_list);
        assert!(!report.styles_rebuilt);
        assert_eq!(layout_id_for_dom(&session.layout().tree.root, from), Some(from_layout));
        assert_eq!(layout_id_for_dom(&session.layout().tree.root, to), Some(to_layout));
        assert_eq!(layout_id_for_dom(&session.layout().tree.root, child), Some(child_layout));
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, existing),
            Some(existing_layout)
        );
        assert_eq!(session.styles(), &expected.styles);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

    #[test]
    fn detach_reflows_retained_old_parent() {
        let source = r#"<div id="parent"><span id="child" style="height:12px;background:#112233">R</span></div><div style="height:10px;background:#445566"></div>"#;
        let expected_source = r#"<div id="parent"></div><div style="height:10px;background:#445566"></div>"#;
        let mut session = session(source, deterministic_options());
        let parent = element_with_id(session.document(), "parent");
        let child = element_with_id(session.document(), "child");
        let parent_layout = layout_id_for_dom(&session.layout().tree.root, parent).unwrap();

        session.document_mut().detach(child).unwrap();
        let report = session.update().expect("detach reflow succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert_eq!(report.patched_nodes, 1);
        assert!(report.retained_display_list);
        assert!(!report.styles_rebuilt);
        assert_eq!(layout_id_for_dom(&session.layout().tree.root, parent), Some(parent_layout));
        assert!(layout_id_for_dom(&session.layout().tree.root, child).is_none());
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

    #[test]
    fn newly_created_then_attached_node_remains_full_rebuild() {
        let source = r#"<div id="parent"></div>"#;
        let expected_source = r#"<div id="parent"><span>R</span></div>"#;
        let mut session = session(source, deterministic_options());
        let parent = element_with_id(session.document(), "parent");
        let child = session
            .document_mut()
            .create_node(NodeKind::Element(rarog_dom::ElementData::html("span")))
            .unwrap();
        session
            .document_mut()
            .append_new(child, NodeKind::Text("R".into()))
            .unwrap();
        session.document_mut().append_child(parent, child).unwrap();

        let report = session.update().expect("created-node fallback succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

'''
    s = s[:start] + tests + s[end:]
engine_path.write_text(s)

correctness_path = Path("crates/rarog-engine/tests/r01_correctness.rs")
c = correctness_path.read_text()
old_name = "fn vertical_flow_append_reflow_and_reparent_fallback_are_preserved() {"
if old_name in c:
    c = c.replace(
        old_name,
        "fn vertical_flow_append_and_reparent_reflow_are_preserved() {",
        1,
    )
    c = c.replace(
        '''    assert_eq!(
        reparent.update().expect("reparent update succeeds").mode,
        IncrementalMode::FullRebuild
    );
    assert_matches_fresh(&reparent, reparent_expected);
''',
        '''    let reparent_report = reparent.update().expect("reparent update succeeds");
    assert_eq!(reparent_report.mode, IncrementalMode::FlowRelayout);
    assert!(reparent_report.retained_display_list);
    assert!(!reparent_report.styles_rebuilt);
    assert_matches_fresh(&reparent, reparent_expected);
''',
        1,
    )
elif "fn vertical_flow_append_and_reparent_reflow_are_preserved()" not in c:
    raise SystemExit("R0.1 reparent expectation marker missing")
correctness_path.write_text(c)
