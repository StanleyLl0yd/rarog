from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
s = path.read_text()

s = s.replace(
'''                    if stylesheet_source_changed {
                        requires_full_rebuild = true;
                    } else if self.document.is_connected(*parent) {
                        structural_relayout_nodes.insert(*parent);
                    }
''',
'''                    if self.document.is_connected(*parent) {
                        structural_relayout_nodes.insert(*parent);
                    }
''',
1,
)

s = s.replace(
'''                    if stylesheet_source_changed {
                        requires_full_rebuild = true;
                    } else {
                        for parent in old_parent.iter().chain(new_parent.iter()) {
                            if self.document.is_connected(*parent) {
                                structural_relayout_nodes.insert(*parent);
                            }
                        }
                    }
''',
'''                    for parent in old_parent.iter().chain(new_parent.iter()) {
                        if self.document.is_connected(*parent) {
                            structural_relayout_nodes.insert(*parent);
                        }
                    }
''',
1,
)

s = s.replace(
'''                MutationKind::CharacterData { node } => {
                    if node_is_within_style_element(&self.document, *node) {
                        requires_full_rebuild = true;
                        stylesheet_sources_changed = true;
                    } else {
                        text_relayout_nodes.insert(*node);
                    }
                }
''',
'''                MutationKind::CharacterData { node } => {
                    if node_is_within_style_element(&self.document, *node) {
                        stylesheet_sources_changed = true;
                    } else {
                        text_relayout_nodes.insert(*node);
                    }
                }
''',
1,
)

structural_marker = '''        if !requires_full_rebuild {
            let mut processed_style_nodes = BTreeSet::new();
'''
revalidation = '''        if !requires_full_rebuild && stylesheet_sources_changed {
            if stylesheet_visibility_boundary_changed_outside_structural_roots(
                &self.document,
                &self.styles,
                &new_styles,
                &self.layout.tree.root,
                &structural_relayout_nodes,
            ) {
                requires_full_rebuild = true;
            } else {
                style_candidates.retain(|candidate| {
                    !node_is_within_style_element(&self.document, *candidate)
                });
                collect_layout_dom_nodes(&self.layout.tree.root, &mut style_candidates);
            }
        }

'''
if structural_marker not in s:
    raise SystemExit("style processing marker missing")
s = s.replace(structural_marker, revalidation + structural_marker, 1)

helper_marker = '''fn layout_style_for_dom(node: &LayoutNode, dom_node: NodeId) -> Option<ComputedStyle> {
'''
helper = '''fn stylesheet_visibility_boundary_changed_outside_structural_roots(
    document: &Document,
    old_styles: &StyleSet,
    new_styles: &StyleSet,
    layout_root: &LayoutNode,
    structural_roots: &BTreeSet<NodeId>,
) -> bool {
    let mut laid_out = BTreeSet::new();
    collect_layout_dom_nodes(layout_root, &mut laid_out);
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
        if matches!(&current.kind, NodeKind::Element(_)) && !laid_out.contains(&node) {
            let old_style = computed_style(document, node, old_styles);
            let new_style = computed_style(document, node, new_styles);
            if old_style.display_none != new_style.display_none {
                return true;
            }
        }
        stack.extend_from_slice(&current.children);
    }
    false
}

'''
if helper_marker not in s:
    raise SystemExit("layout style helper marker missing")
s = s.replace(helper_marker, helper + helper_marker, 1)

old_insert_test = '''        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert!(report.styles_rebuilt);
        assert_eq!(session.styles(), &expected.styles);
'''
new_insert_test = '''        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert!(report.retained_display_list);
        assert!(report.styles_rebuilt);
        assert_eq!(session.styles(), &expected.styles);
'''
if old_insert_test not in s:
    raise SystemExit("style insertion expectation marker missing")
s = s.replace(old_insert_test, new_insert_test, 1)

test_marker = '''    #[test]
    fn inserting_style_subtree_rebuilds_style_sources() {
'''
tests = r'''    #[test]
    fn stylesheet_text_paint_change_revalidates_retained_layout() {
        let source = r#"<style id="sheet">#target { width:80px;height:20px;background:#112233; }</style><div id="target">R</div>"#;
        let expected_source = r#"<style id="sheet">#target { width:80px;height:20px;background:#445566; }</style><div id="target">R</div>"#;
        let mut session = session(source, deterministic_options());
        let sheet = element_with_id(session.document(), "sheet");
        let target = element_with_id(session.document(), "target");
        let text = *session
            .document()
            .children(sheet)
            .and_then(|children| children.first())
            .expect("style element contains text");
        let target_layout = layout_id_for_dom(&session.layout().tree.root, target).unwrap();

        session
            .document_mut()
            .set_text(text, "#target { width:80px;height:20px;background:#445566; }")
            .unwrap();
        let report = session.update().expect("stylesheet paint revalidation succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
        assert!(report.retained_display_list);
        assert!(report.styles_rebuilt);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, target),
            Some(target_layout)
        );
        assert_eq!(session.styles(), &expected.styles);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

    #[test]
    fn stylesheet_text_geometry_change_uses_retained_flow_relayout() {
        let source = r#"<style id="sheet">#target { height:20px;background:#112233; }</style><div id="target"></div><div id="after" style="height:10px;background:#445566"></div>"#;
        let expected_source = r#"<style id="sheet">#target { height:32px;background:#112233; }</style><div id="target"></div><div id="after" style="height:10px;background:#445566"></div>"#;
        let mut session = session(source, deterministic_options());
        let sheet = element_with_id(session.document(), "sheet");
        let target = element_with_id(session.document(), "target");
        let text = *session
            .document()
            .children(sheet)
            .and_then(|children| children.first())
            .expect("style element contains text");
        let target_layout = layout_id_for_dom(&session.layout().tree.root, target).unwrap();

        session
            .document_mut()
            .set_text(text, "#target { height:32px;background:#112233; }")
            .unwrap();
        let report = session.update().expect("stylesheet geometry revalidation succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert!(report.retained_display_list);
        assert!(report.styles_rebuilt);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, target),
            Some(target_layout)
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

    #[test]
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
            .set_text(text, "#target { display:none;height:20px;background:#112233; }")
            .unwrap();
        let report = session.update().expect("stylesheet boundary fallback succeeds");
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
if test_marker not in s:
    raise SystemExit("style insertion test marker missing")
s = s.replace(test_marker, tests + test_marker, 1)

path.write_text(s)

correctness_path = Path("crates/rarog-engine/tests/r01_correctness.rs")
c = correctness_path.read_text()
insert = '''#[test]
fn structural_limits_reject_deep_and_wide_documents_before_recursive_rendering() {
'''
addition = r'''#[test]
fn stylesheet_source_revalidation_matches_fresh_render() {
    let source = r#"<style id="sheet">#target { height:20px;background:#112233; }</style><div id="target"></div><div style="height:10px;background:#445566"></div>"#;
    let expected = r#"<style id="sheet">#target { height:32px;background:#778899; }</style><div id="target"></div><div style="height:10px;background:#445566"></div>"#;
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let sheet = element_with_id(session.document(), "sheet");
    let text = *session
        .document()
        .children(sheet)
        .and_then(|children| children.first())
        .expect("style element contains text");
    session
        .document_mut()
        .set_text(text, "#target { height:32px;background:#778899; }")
        .expect("stylesheet mutation succeeds");

    let report = session.update().expect("stylesheet revalidation succeeds");
    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert!(report.retained_display_list);
    assert!(report.styles_rebuilt);
    assert_matches_fresh(&session, expected);
}

#[test]
fn structural_limits_reject_deep_and_wide_documents_before_recursive_rendering() {
'''
if insert not in c:
    raise SystemExit("R0.1 insertion marker missing")
c = c.replace(insert, addition, 1)
correctness_path.write_text(c)
