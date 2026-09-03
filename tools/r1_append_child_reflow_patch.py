from pathlib import Path

layout_path = Path("crates/rarog-layout/src/lib.rs")
layout = layout_path.read_text()

layout_marker = '''pub fn fragment_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Option<&Fragment> {
'''
layout_code = '''pub fn refresh_layout_subtree(
    tree: &mut LayoutTree,
    document: &Document,
    styles: &StyleSet,
    dom_node: NodeId,
) -> bool {
    let next_id = max_layout_node_id(&tree.root).saturating_add(1);
    refresh_layout_subtree_recursive(
        &mut tree.root,
        document,
        styles,
        dom_node,
        None,
        next_id,
    )
}

fn refresh_layout_subtree_recursive(
    node: &mut LayoutNode,
    document: &Document,
    styles: &StyleSet,
    dom_node: NodeId,
    parent_style: Option<ComputedStyle>,
    next_id: usize,
) -> bool {
    if node.dom_node == Some(dom_node) {
        let mut retained_ids = std::collections::BTreeMap::new();
        collect_layout_node_ids(node, &mut retained_ids);
        let mut builder = LayoutTreeBuilder { next_id, styles };
        let Some(mut replacement) = builder.build_node(document, dom_node, parent_style) else {
            return false;
        };
        reuse_layout_node_ids(&mut replacement, &retained_ids);
        *node = replacement;
        return true;
    }

    let style = node.style;
    let changed = node.children.iter_mut().any(|child| {
        refresh_layout_subtree_recursive(
            child,
            document,
            styles,
            dom_node,
            Some(style),
            next_id,
        )
    });
    if changed {
        node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
    }
    changed
}

fn max_layout_node_id(node: &LayoutNode) -> usize {
    node.children
        .iter()
        .map(max_layout_node_id)
        .fold(node.id.index(), usize::max)
}

fn collect_layout_node_ids(
    node: &LayoutNode,
    ids: &mut std::collections::BTreeMap<NodeId, LayoutNodeId>,
) {
    if let Some(dom_node) = node.dom_node {
        ids.insert(dom_node, node.id);
    }
    for child in &node.children {
        collect_layout_node_ids(child, ids);
    }
}

fn reuse_layout_node_ids(
    node: &mut LayoutNode,
    ids: &std::collections::BTreeMap<NodeId, LayoutNodeId>,
) {
    if let Some(dom_node) = node.dom_node {
        if let Some(id) = ids.get(&dom_node) {
            node.id = *id;
        }
    }
    for child in &mut node.children {
        reuse_layout_node_ids(child, ids);
    }
}

'''
if "pub fn refresh_layout_subtree(" not in layout:
    if layout_marker not in layout:
        raise SystemExit("layout insertion marker missing")
    layout = layout.replace(layout_marker, layout_code + layout_marker, 1)
layout_path.write_text(layout)

engine_path = Path("crates/rarog-engine/src/lib.rs")
s = engine_path.read_text()

old_import = '''    fragment_for_dom, fragments_for_dom, layout_document_with_styles, refresh_text_node,
    relayout_fragment_flow, relayout_fragment_subtree, relayout_tree,
'''
new_import = '''    fragment_for_dom, fragments_for_dom, layout_document_with_styles, refresh_layout_subtree,
    refresh_text_node, relayout_fragment_flow, relayout_fragment_subtree, relayout_tree,
'''
if old_import in s:
    s = s.replace(old_import, new_import, 1)
elif "refresh_layout_subtree" not in s:
    raise SystemExit("engine layout import marker missing")

old_sets = '''        let mut text_relayout_nodes = BTreeSet::new();
        let mut requires_full_rebuild = mutation_history_lost;
'''
new_sets = '''        let mut text_relayout_nodes = BTreeSet::new();
        let mut structural_relayout_nodes = BTreeSet::new();
        let mut requires_full_rebuild = mutation_history_lost;
'''
if old_sets in s:
    s = s.replace(old_sets, new_sets, 1)
elif "let mut structural_relayout_nodes" not in s:
    raise SystemExit("engine structural set marker missing")

old_structural = '''                MutationKind::NodeCreated { .. } => {
                    requires_full_rebuild = true;
                }
                MutationKind::ChildAdded { parent, child } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed |= self.document.is_connected(*parent)
                        && (node_is_within_style_element(&self.document, *parent)
                            || subtree_contains_style_element(&self.document, *child));
                }
'''
new_structural = '''                MutationKind::NodeCreated { node } => {
                    if self.document.is_connected(*node) {
                        requires_full_rebuild = true;
                    }
                }
                MutationKind::ChildAdded { parent, child } => {
                    let stylesheet_source_changed = self.document.is_connected(*parent)
                        && (node_is_within_style_element(&self.document, *parent)
                            || subtree_contains_style_element(&self.document, *child));
                    stylesheet_sources_changed |= stylesheet_source_changed;
                    if stylesheet_source_changed {
                        requires_full_rebuild = true;
                    } else if self.document.is_connected(*parent) {
                        structural_relayout_nodes.insert(*parent);
                    }
                }
'''
if old_structural in s:
    s = s.replace(old_structural, new_structural, 1)
elif "structural_relayout_nodes.insert(*parent)" not in s:
    raise SystemExit("engine ChildAdded marker missing")

flow_decl = '''        let mut flow_relayout_nodes = BTreeSet::new();

        if !requires_full_rebuild {
'''
structural_refresh = '''        let mut flow_relayout_nodes = BTreeSet::new();

        if !requires_full_rebuild && !structural_relayout_nodes.is_empty() {
            for node in structural_relayout_nodes.iter().copied() {
                if !refresh_layout_subtree(&mut self.layout.tree, &self.document, &new_styles, node) {
                    requires_full_rebuild = true;
                    break;
                }
                geometry_changed = true;
                subtree_relayout_safe = false;
                flow_relayout_nodes.insert(node);
            }
            if !requires_full_rebuild {
                style_candidates.retain(|candidate| {
                    !structural_relayout_nodes.iter().any(|root| {
                        node_is_within_dom_subtree(&self.document, *root, *candidate)
                    })
                });
            }
        }

        if !requires_full_rebuild {
'''
if flow_decl in s:
    s = s.replace(flow_decl, structural_refresh, 1)
elif "node_is_within_dom_subtree(&self.document" not in s:
    raise SystemExit("engine structural refresh marker missing")

old_patched = '''            patched_nodes = style_updates.len() + text_relayout_nodes.len();
'''
new_patched = '''            patched_nodes =
                style_updates.len() + text_relayout_nodes.len() + structural_relayout_nodes.len();
'''
if old_patched in s:
    s = s.replace(old_patched, new_patched, 1)
elif "structural_relayout_nodes.len()" not in s:
    raise SystemExit("engine patched-node marker missing")

helper_marker = '''fn subtree_contains_style_element(document: &Document, root: NodeId) -> bool {
'''
helper = '''fn node_is_within_dom_subtree(document: &Document, root: NodeId, mut node: NodeId) -> bool {
    let mut remaining = document.node_count().saturating_add(1);
    while remaining > 0 {
        if node == root {
            return true;
        }
        let Some(parent) = document.node(node).and_then(|node| node.parent) else {
            return false;
        };
        node = parent;
        remaining -= 1;
    }
    false
}

'''
if "fn node_is_within_dom_subtree(" not in s:
    if helper_marker not in s:
        raise SystemExit("engine DOM-subtree helper marker missing")
    s = s.replace(helper_marker, helper + helper_marker, 1)

# Test helper for retained LayoutNode identity.
element_helper_marker = '''    #[test]
    fn full_render_exposes_stage_observability_without_affecting_identity() {
'''
layout_helper = '''    fn layout_id_for_dom(
        node: &LayoutNode,
        dom_node: NodeId,
    ) -> Option<rarog_layout::LayoutNodeId> {
        if node.dom_node == Some(dom_node) {
            return Some(node.id);
        }
        node.children
            .iter()
            .find_map(|child| layout_id_for_dom(child, dom_node))
    }

'''
if "fn layout_id_for_dom(" not in s:
    if element_helper_marker not in s:
        raise SystemExit("engine test helper marker missing")
    s = s.replace(element_helper_marker, layout_helper + element_helper_marker, 1)

structural_test_marker = '''    #[test]
    fn ordinary_structural_change_reuses_existing_style_set() {
'''
structural_test = r'''    #[test]
    fn child_added_reflows_retained_layout_subtree() {
        let source = "<style>#parent > div:last-child { height:12px; background:#112233; }</style><div id=\"before\" style=\"height:5px;background:#eeeeee\"></div><div id=\"parent\"><div id=\"first\"></div></div><div id=\"after\" style=\"height:10px;background:#445566\"></div>";
        let expected_source = "<style>#parent > div:last-child { height:12px; background:#112233; }</style><div id=\"before\" style=\"height:5px;background:#eeeeee\"></div><div id=\"parent\"><div id=\"first\"></div><div></div></div><div id=\"after\" style=\"height:10px;background:#445566\"></div>";
        let mut session = session(source, deterministic_options());
        let parent = element_with_id(session.document(), "parent");
        let first = element_with_id(session.document(), "first");
        let parent_layout_id = layout_id_for_dom(&session.layout().tree.root, parent).unwrap();
        let first_layout_id = layout_id_for_dom(&session.layout().tree.root, first).unwrap();

        let added = session
            .document_mut()
            .append_new(
                parent,
                NodeKind::Element(rarog_dom::ElementData::html("div")),
            )
            .unwrap();

        let report = session.update().expect("append-only structural reflow succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert_eq!(report.patched_nodes, 1);
        assert!(report.retained_display_list);
        assert!(!report.styles_rebuilt);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout_id)
        );
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, first),
            Some(first_layout_id)
        );
        assert!(layout_id_for_dom(&session.layout().tree.root, added).is_some());
        assert_eq!(session.styles(), &expected.styles);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
        assert!(!session.damage().rects.is_empty());
    }

'''
if "fn child_added_reflows_retained_layout_subtree()" not in s:
    if structural_test_marker not in s:
        raise SystemExit("engine structural test marker missing")
    s = s.replace(structural_test_marker, structural_test + structural_test_marker, 1)

engine_path.write_text(s)
