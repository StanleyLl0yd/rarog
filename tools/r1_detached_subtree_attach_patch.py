from pathlib import Path

engine_path = Path("crates/rarog-engine/src/lib.rs")
s = engine_path.read_text()

old_decl = '''        let mut text_relayout_nodes = BTreeSet::new();
        let mut structural_relayout_nodes = BTreeSet::new();
        let mut requires_full_rebuild = mutation_history_lost;
'''
new_decl = '''        let mut text_relayout_nodes = BTreeSet::new();
        let mut structural_relayout_nodes = BTreeSet::new();
        let mut connected_created_nodes = BTreeSet::new();
        let mut requires_full_rebuild = mutation_history_lost;
'''
if old_decl not in s:
    raise SystemExit("declaration marker missing")
s = s.replace(old_decl, new_decl, 1)

old_created = '''                MutationKind::NodeCreated { node } => {
                    if self.document.is_connected(*node) {
                        requires_full_rebuild = true;
                    }
                }
'''
new_created = '''                MutationKind::NodeCreated { node } => {
                    if self.document.is_connected(*node) {
                        connected_created_nodes.insert(*node);
                    }
                }
'''
if old_created not in s:
    raise SystemExit("NodeCreated marker missing")
s = s.replace(old_created, new_created, 1)

insert_marker = '''        let new_styles = if stylesheet_sources_changed {
'''
guard = '''        if !requires_full_rebuild && !connected_created_nodes.is_empty() {
            let created_nodes_are_covered = connected_created_nodes.iter().all(|created| {
                structural_relayout_nodes.iter().any(|root| {
                    node_is_within_dom_subtree(&self.document, *root, *created)
                })
            });
            if !created_nodes_are_covered {
                requires_full_rebuild = true;
            }
        }

'''
if insert_marker not in s:
    raise SystemExit("new_styles marker missing")
s = s.replace(insert_marker, guard + insert_marker, 1)

old_retain = '''                style_candidates.retain(|candidate| {
                    !structural_relayout_nodes
                        .iter()
                        .any(|root| node_is_within_dom_subtree(&self.document, *root, *candidate))
                });
'''
new_retain = '''                style_candidates.retain(|candidate| {
                    !structural_relayout_nodes
                        .iter()
                        .any(|root| node_is_within_dom_subtree(&self.document, *root, *candidate))
                });
                text_relayout_nodes.retain(|candidate| {
                    !structural_relayout_nodes
                        .iter()
                        .any(|root| node_is_within_dom_subtree(&self.document, *root, *candidate))
                });
'''
if old_retain not in s:
    raise SystemExit("structural retain marker missing")
s = s.replace(old_retain, new_retain, 1)

start = s.find("    #[test]\n    fn newly_created_then_attached_node_remains_full_rebuild() {")
end = s.find("    #[test]\n    fn unrelated_attribute_change_does_not_rebuild_render_state() {", start)
if start < 0 or end < 0:
    raise SystemExit("created-node fallback test marker missing")
replacement = r'''    #[test]
    fn detached_created_subtree_attaches_through_retained_parent() {
        let source = r#"<div id="before" style="height:5px;background:#eeeeee"></div><div id="parent"></div><div id="after" style="height:10px;background:#445566"></div>"#;
        let expected_source = r#"<div id="before" style="height:5px;background:#eeeeee"></div><div id="parent"><section id="card" style="height:12px;background:#112233"><span id="label">R</span></section></div><div id="after" style="height:10px;background:#445566"></div>"#;
        let mut session = session(source, deterministic_options());
        let before = element_with_id(session.document(), "before");
        let parent = element_with_id(session.document(), "parent");
        let parent_layout = layout_id_for_dom(&session.layout().tree.root, parent).unwrap();
        let before_fragment = fragment_for_dom(&session.layout().fragments, before)
            .expect("retained prefix fragment exists")
            .id;

        let card = session
            .document_mut()
            .create_node(NodeKind::Element(rarog_dom::ElementData::html("section")))
            .unwrap();
        session.document_mut().set_attribute(card, "id", "card").unwrap();
        session
            .document_mut()
            .set_attribute(card, "style", "height:12px;background:#112233")
            .unwrap();
        let label = session
            .document_mut()
            .create_node(NodeKind::Element(rarog_dom::ElementData::html("span")))
            .unwrap();
        session.document_mut().set_attribute(label, "id", "label").unwrap();
        let text = session
            .document_mut()
            .create_node(NodeKind::Text("R".into()))
            .unwrap();
        session.document_mut().append_child(label, text).unwrap();
        session.document_mut().append_child(card, label).unwrap();
        session.document_mut().append_child(parent, card).unwrap();

        let report = session.update().expect("detached subtree attach succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert_eq!(report.patched_nodes, 1);
        assert!(report.retained_display_list);
        assert!(!report.styles_rebuilt);
        assert_eq!(
            layout_id_for_dom(&session.layout().tree.root, parent),
            Some(parent_layout)
        );
        assert!(layout_id_for_dom(&session.layout().tree.root, card).is_some());
        assert!(layout_id_for_dom(&session.layout().tree.root, label).is_some());
        assert!(layout_id_for_dom(&session.layout().tree.root, text).is_some());
        assert_eq!(
            fragment_for_dom(&session.layout().fragments, before)
                .expect("retained prefix fragment remains")
                .id,
            before_fragment
        );
        assert_eq!(session.styles(), &expected.styles);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
        assert!(!session.damage().rects.is_empty());
    }

'''
s = s[:start] + replacement + s[end:]
engine_path.write_text(s)

correctness_path = Path("crates/rarog-engine/tests/r01_correctness.rs")
c = correctness_path.read_text()
marker = '''    assert_matches_fresh(&reparent, reparent_expected);
}

#[test]
fn structural_limits_reject_deep_and_wide_documents_before_recursive_rendering() {
'''
addition = r'''    assert_matches_fresh(&reparent, reparent_expected);

    let attach_source = r#"<div id="host"></div>"#;
    let attach_expected = r#"<div id="host"><span id="created">R</span></div>"#;
    let mut attach = RenderSession::new(attach_source, options()).expect("session starts");
    let host = element_with_id(attach.document(), "host");
    let created = attach
        .document_mut()
        .create_node(NodeKind::Element(ElementData::html("span")))
        .expect("detached element creation succeeds");
    attach
        .document_mut()
        .set_attribute(created, "id", "created")
        .expect("detached attribute mutation succeeds");
    let text = attach
        .document_mut()
        .create_node(NodeKind::Text("R".into()))
        .expect("detached text creation succeeds");
    attach
        .document_mut()
        .append_child(created, text)
        .expect("detached subtree construction succeeds");
    attach
        .document_mut()
        .append_child(host, created)
        .expect("detached subtree attachment succeeds");
    let attach_report = attach.update().expect("attach update succeeds");
    assert_eq!(attach_report.mode, IncrementalMode::FlowRelayout);
    assert!(attach_report.retained_display_list);
    assert!(!attach_report.styles_rebuilt);
    assert_matches_fresh(&attach, attach_expected);
}

#[test]
fn structural_limits_reject_deep_and_wide_documents_before_recursive_rendering() {
'''
if marker not in c:
    raise SystemExit("R0.1 attach marker missing")
c = c.replace(marker, addition, 1)
correctness_path.write_text(c)
