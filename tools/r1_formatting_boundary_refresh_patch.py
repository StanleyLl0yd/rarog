from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
s = path.read_text()

old = '''        let mut text_relayout_nodes = BTreeSet::new();
        let mut structural_relayout_nodes = BTreeSet::new();
        let mut connected_created_nodes = BTreeSet::new();
'''
new = '''        let mut text_relayout_nodes = BTreeSet::new();
        let mut structural_relayout_nodes = BTreeSet::new();
        let mut formatting_relayout_nodes = BTreeSet::new();
        let mut connected_created_nodes = BTreeSet::new();
'''
if old not in s:
    raise SystemExit("incremental set marker missing")
s = s.replace(old, new, 1)

old = '''        if !requires_full_rebuild && stylesheet_sources_changed {
            if stylesheet_visibility_boundary_changed_outside_structural_roots(
                &self.document,
                &self.styles,
                &new_styles,
                &self.layout.tree.root,
                &structural_relayout_nodes,
            ) {
                requires_full_rebuild = true;
            } else {
                style_candidates
                    .retain(|candidate| !node_is_within_style_element(&self.document, *candidate));
                collect_layout_dom_nodes(&self.layout.tree.root, &mut style_candidates);
            }
        }
'''
new = '''        if !requires_full_rebuild && stylesheet_sources_changed {
            if !collect_stylesheet_formatting_boundary_roots(
                &self.document,
                &self.styles,
                &new_styles,
                &self.layout.tree.root,
                &structural_relayout_nodes,
                &mut formatting_relayout_nodes,
            ) {
                requires_full_rebuild = true;
            } else {
                style_candidates
                    .retain(|candidate| !node_is_within_style_element(&self.document, *candidate));
                collect_layout_dom_nodes(&self.layout.tree.root, &mut style_candidates);
            }
        }
'''
if old not in s:
    raise SystemExit("stylesheet boundary marker missing")
s = s.replace(old, new, 1)

old = '''                let Some(old_style) = layout_style_for_dom(&self.layout.tree.root, node) else {
                    requires_full_rebuild = true;
                    break;
                };
                let new_style = computed_style(&self.document, node, &new_styles);
                if old_style.display_none != new_style.display_none
                    || old_style.display_inline != new_style.display_inline
                    || old_style.establishes_bfc != new_style.establishes_bfc
                {
                    requires_full_rebuild = true;
                    break;
                }
'''
new = '''                let new_style = computed_style(&self.document, node, &new_styles);
                let Some(old_style) = layout_style_for_dom(&self.layout.tree.root, node) else {
                    let Some(current) = self.document.node(node) else {
                        requires_full_rebuild = true;
                        break;
                    };
                    if node_is_within_style_element(&self.document, node) {
                        continue;
                    }
                    if matches!(current.kind, NodeKind::Element(_)) && current.parent.is_some() {
                        if new_style.display_none {
                            continue;
                        }
                        let Some(root) = retained_structural_parent(
                            &self.document,
                            &self.layout.tree.root,
                            node,
                        ) else {
                            requires_full_rebuild = true;
                            break;
                        };
                        formatting_relayout_nodes.insert(root);
                        continue;
                    }
                    requires_full_rebuild = true;
                    break;
                };
                if formatting_boundary_changed(old_style, new_style) {
                    let Some(root) = retained_structural_parent(
                        &self.document,
                        &self.layout.tree.root,
                        node,
                    ) else {
                        requires_full_rebuild = true;
                        break;
                    };
                    formatting_relayout_nodes.insert(root);
                    continue;
                }
'''
if old not in s:
    raise SystemExit("style boundary fallback marker missing")
s = s.replace(old, new, 1)

marker = '''        if !requires_full_rebuild {
            for node in &text_relayout_nodes {
'''
insert = '''        if !requires_full_rebuild && !formatting_relayout_nodes.is_empty() {
            formatting_relayout_nodes =
                minimal_structural_roots(&self.document, &formatting_relayout_nodes);
            let formatting_roots = formatting_relayout_nodes
                .iter()
                .copied()
                .collect::<Vec<_>>();
            if !refresh_layout_subtrees(
                &mut self.layout.tree,
                &self.document,
                &new_styles,
                &formatting_roots,
            ) {
                requires_full_rebuild = true;
            } else {
                geometry_changed = true;
                subtree_relayout_safe = false;
                flow_relayout_nodes.extend(formatting_roots.iter().copied());
                structural_relayout_nodes.extend(formatting_roots.iter().copied());
                style_updates.retain(|(candidate, _)| {
                    !formatting_relayout_nodes.iter().any(|root| {
                        node_is_within_dom_subtree(&self.document, *root, *candidate)
                    })
                });
                text_relayout_nodes.retain(|candidate| {
                    !formatting_relayout_nodes.iter().any(|root| {
                        node_is_within_dom_subtree(&self.document, *root, *candidate)
                    })
                });
            }
        }

        if !requires_full_rebuild {
            for node in &text_relayout_nodes {
'''
if marker not in s:
    raise SystemExit("text refresh marker missing")
s = s.replace(marker, insert, 1)

start = s.find("fn stylesheet_visibility_boundary_changed_outside_structural_roots(\n")
end = s.find("fn layout_style_for_dom(", start)
if start < 0 or end < 0:
    raise SystemExit("stylesheet visibility helper markers missing")
replacement = '''fn formatting_boundary_changed(before: ComputedStyle, after: ComputedStyle) -> bool {
    before.display_none != after.display_none
        || before.display_inline != after.display_inline
        || before.establishes_bfc != after.establishes_bfc
}

fn retained_structural_parent(
    document: &Document,
    layout_root: &LayoutNode,
    node: NodeId,
) -> Option<NodeId> {
    let mut current = document.node(node)?.parent?;
    let mut remaining = document.node_count().saturating_add(1);
    while remaining > 0 {
        if layout_style_for_dom(layout_root, current).is_some() {
            return Some(current);
        }
        current = document.node(current)?.parent?;
        remaining -= 1;
    }
    None
}

fn collect_stylesheet_formatting_boundary_roots(
    document: &Document,
    old_styles: &StyleSet,
    new_styles: &StyleSet,
    layout_root: &LayoutNode,
    structural_roots: &BTreeSet<NodeId>,
    output: &mut BTreeSet<NodeId>,
) -> bool {
    let mut stack = vec![document.root()];

    while let Some(node) = stack.pop() {
        if structural_roots
            .iter()
            .any(|root| node_is_within_dom_subtree(document, *root, node))
        {
            continue;
        }
        let Some(current) = document.node(node) else {
            continue;
        };
        if matches!(&current.kind, NodeKind::Element(_))
            && !node_is_within_style_element(document, node)
        {
            let old_style = computed_style(document, node, old_styles);
            let new_style = computed_style(document, node, new_styles);
            if formatting_boundary_changed(old_style, new_style) {
                let Some(root) = retained_structural_parent(document, layout_root, node) else {
                    return false;
                };
                output.insert(root);
            }
        }
        stack.extend_from_slice(&current.children);
    }
    true
}

'''
s = s[:start] + replacement + s[end:]

old_test = '''    #[test]
    fn stylesheet_visibility_boundary_change_remains_full_rebuild() {
        let source = r#"<style id="sheet">#target { display:block;height:20px;background:#112233; }</style><div id="target"></div>"#;
        let expected_source = r#"<style id="sheet">#target { display:none;height:20px;background:#112233; }</style><div id="target"></div>"#;
        let mut session = session(source, deterministic_options());
        let sheet = element_with_id(session.document(), "sheet");
        let text = *session
            .document()
            .children(sheet)
            .and_then(|children| children.first())
            .expect("style element contains text");

        session
            .document_mut()
            .set_text(
                text,
                "#target { display:none;height:20px;background:#112233; }",
            )
            .unwrap();
        let report = session
            .update()
            .expect("stylesheet boundary fallback succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert!(!report.retained_display_list);
        assert!(report.styles_rebuilt);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }
'''
new_tests = '''    #[test]
    fn stylesheet_visibility_boundary_change_refreshes_retained_parent() {
        let source = r#"<style id="sheet">#target { display:block;height:20px;background:#112233; }</style><div id="before" style="height:5px"></div><div id="parent"><div id="target"></div></div>"#;
        let expected_source = r#"<style id="sheet">#target { display:none;height:20px;background:#112233; }</style><div id="before" style="height:5px"></div><div id="parent"><div id="target"></div></div>"#;
        let mut session = session(source, deterministic_options());
        let sheet = element_with_id(session.document(), "sheet");
        let parent = element_with_id(session.document(), "parent");
        let target = element_with_id(session.document(), "target");
        let parent_layout = layout_id_for_dom(&session.layout().tree.root, parent).unwrap();
        let text = *session
            .document()
            .children(sheet)
            .and_then(|children| children.first())
            .expect("style element contains text");

        session
            .document_mut()
            .set_text(
                text,
                "#target { display:none;height:20px;background:#112233; }",
            )
            .unwrap();
        let report = session
            .update()
            .expect("stylesheet boundary refresh succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert!(report.retained_display_list);
        assert!(report.styles_rebuilt);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout)
        );
        assert!(layout_id_for_dom(&session.layout().tree.root, target).is_none());
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

    #[test]
    fn direct_visibility_boundary_changes_refresh_retained_parent() {
        let source = r#"<div id="before" style="height:5px;background:#eeeeee"></div><div id="parent"><div id="target" style="height:12px;background:#112233"></div></div><div id="after" style="height:10px;background:#445566"></div>"#;
        let hidden_source = r#"<div id="before" style="height:5px;background:#eeeeee"></div><div id="parent"><div id="target" style="display:none;height:12px;background:#112233"></div></div><div id="after" style="height:10px;background:#445566"></div>"#;
        let mut session = session(source, deterministic_options());
        let before = element_with_id(session.document(), "before");
        let parent = element_with_id(session.document(), "parent");
        let target = element_with_id(session.document(), "target");
        let parent_layout = layout_id_for_dom(&session.layout().tree.root, parent).unwrap();
        let before_fragment = fragment_for_dom(&session.layout().fragments, before)
            .expect("prefix fragment exists")
            .id;

        session
            .document_mut()
            .set_attribute(
                target,
                "style",
                "display:none;height:12px;background:#112233",
            )
            .unwrap();
        let hide_report = session.update().expect("hide refresh succeeds");
        let hidden = render_ok(hidden_source, deterministic_options());

        assert_eq!(hide_report.mode, IncrementalMode::FlowRelayout);
        assert!(hide_report.retained_display_list);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout)
        );
        assert!(layout_id_for_dom(&session.layout().tree.root, target).is_none());
        assert_eq!(
            fragment_for_dom(&session.layout().fragments, before)
                .expect("prefix fragment remains")
                .id,
            before_fragment
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            hidden.framebuffer.stable_hash64()
        );

        session
            .document_mut()
            .set_attribute(target, "style", "height:12px;background:#112233")
            .unwrap();
        let show_report = session.update().expect("show refresh succeeds");
        let visible = render_ok(source, deterministic_options());

        assert_eq!(show_report.mode, IncrementalMode::FlowRelayout);
        assert!(show_report.retained_display_list);
        assert!(layout_id_for_dom(&session.layout().tree.root, target).is_some());
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout)
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            visible.framebuffer.stable_hash64()
        );
    }

    #[test]
    fn display_role_and_bfc_changes_refresh_retained_parent() {
        let source = r#"<div id="parent"><div id="target" style="display:block;background:#112233">R</div><span id="sibling">S</span></div>"#;
        let inline_source = r#"<div id="parent"><div id="target" style="display:inline;background:#112233">R</div><span id="sibling">S</span></div>"#;
        let flow_root_source = r#"<div id="parent"><div id="target" style="display:flow-root;background:#112233">R</div><span id="sibling">S</span></div>"#;
        let mut session = session(source, deterministic_options());
        let parent = element_with_id(session.document(), "parent");
        let target = element_with_id(session.document(), "target");
        let parent_layout = layout_id_for_dom(&session.layout().tree.root, parent).unwrap();

        session
            .document_mut()
            .set_attribute(target, "style", "display:inline;background:#112233")
            .unwrap();
        let inline_report = session.update().expect("inline-role refresh succeeds");
        let inline = render_ok(inline_source, deterministic_options());

        assert_eq!(inline_report.mode, IncrementalMode::FlowRelayout);
        assert!(inline_report.retained_display_list);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout)
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            inline.framebuffer.stable_hash64()
        );

        session
            .document_mut()
            .set_attribute(target, "style", "display:flow-root;background:#112233")
            .unwrap();
        let bfc_report = session.update().expect("BFC refresh succeeds");
        let flow_root = render_ok(flow_root_source, deterministic_options());

        assert_eq!(bfc_report.mode, IncrementalMode::FlowRelayout);
        assert!(bfc_report.retained_display_list);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout)
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            flow_root.framebuffer.stable_hash64()
        );
    }
'''
if old_test not in s:
    raise SystemExit("stylesheet visibility test marker missing")
s = s.replace(old_test, new_tests, 1)

path.write_text(s)
