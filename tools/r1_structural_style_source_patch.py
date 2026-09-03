from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
s = path.read_text()

if "pub styles_rebuilt: bool," not in s:
    s = s.replace(
        "    pub retained_display_list: bool,\n    pub elapsed: Duration,\n",
        "    pub retained_display_list: bool,\n    pub styles_rebuilt: bool,\n    pub elapsed: Duration,\n",
        1,
    )
    s = s.replace(
        "                retained_display_list: true,\n                elapsed: update_started.elapsed(),\n",
        "                retained_display_list: true,\n                styles_rebuilt: false,\n                elapsed: update_started.elapsed(),\n",
        1,
    )
    s = s.replace(
        "            retained_display_list,\n            elapsed: update_started.elapsed(),\n",
        "            retained_display_list,\n            styles_rebuilt: stylesheet_sources_changed,\n            elapsed: update_started.elapsed(),\n",
        1,
    )

old_structural = '''                MutationKind::NodeCreated { .. }
                | MutationKind::ChildAdded { .. }
                | MutationKind::Reparented { .. } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed = true;
                }
'''
new_structural = '''                MutationKind::NodeCreated { .. } => {
                    requires_full_rebuild = true;
                }
                MutationKind::ChildAdded { parent, child } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed |= self.document.is_connected(*parent)
                        && (node_is_within_style_element(&self.document, *parent)
                            || subtree_contains_style_element(&self.document, *child));
                }
                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed |= subtree_contains_style_element(&self.document, *child)
                        || old_parent.is_some_and(|parent| {
                            node_is_within_style_element(&self.document, parent)
                        })
                        || new_parent.is_some_and(|parent| {
                            node_is_within_style_element(&self.document, parent)
                        });
                }
'''
if old_structural in s:
    s = s.replace(old_structural, new_structural, 1)
elif "subtree_contains_style_element(&self.document, *child)" not in s:
    raise SystemExit("structural mutation marker missing")

helper_marker = "fn layout_style_for_dom(node: &LayoutNode, dom_node: NodeId) -> Option<ComputedStyle> {\n"
helper = '''fn subtree_contains_style_element(document: &Document, root: NodeId) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let Some(current) = document.node(node) else {
            continue;
        };
        if matches!(&current.kind, NodeKind::Element(element) if element.tag_name.as_str() == "style") {
            return true;
        }
        stack.extend_from_slice(&current.children);
    }
    false
}

'''
if "fn subtree_contains_style_element(" not in s:
    if helper_marker not in s:
        raise SystemExit("style helper marker missing")
    s = s.replace(helper_marker, helper + helper_marker, 1)

test_marker = "    #[test]\n    fn structural_change_still_falls_back_to_full_rebuild() {\n"
tests = r'''    #[test]
    fn ordinary_structural_change_reuses_existing_style_set() {
        let source = "<style>.card { background:#112233; }</style><div id=\"parent\" class=\"card\"></div>";
        let expected_source = "<style>.card { background:#112233; }</style><div id=\"parent\" class=\"card\"><span></span></div>";
        let mut session = session(source, deterministic_options());
        let parent = element_with_id(session.document(), "parent");

        session
            .document_mut()
            .append_new(
                parent,
                NodeKind::Element(rarog_dom::ElementData::html("span")),
            )
            .unwrap();
        let report = session.update().expect("structural update succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert!(!report.styles_rebuilt);
        assert_eq!(session.styles(), &expected.styles);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

    #[test]
    fn inserting_style_subtree_rebuilds_style_sources() {
        let source = "<div id=\"parent\" style=\"height:20px\"></div>";
        let expected_source = "<div id=\"parent\" style=\"height:20px\"><style>#parent { background:#445566; }</style></div>";
        let mut session = session(source, deterministic_options());
        let parent = element_with_id(session.document(), "parent");
        let style = session
            .document_mut()
            .append_new(
                parent,
                NodeKind::Element(rarog_dom::ElementData::html("style")),
            )
            .unwrap();
        session
            .document_mut()
            .append_new(style, NodeKind::Text("#parent { background:#445566; }".into()))
            .unwrap();

        let report = session.update().expect("style insertion succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert!(report.styles_rebuilt);
        assert_eq!(session.styles(), &expected.styles);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }

'''
if "fn ordinary_structural_change_reuses_existing_style_set()" not in s:
    if test_marker not in s:
        raise SystemExit("structural test marker missing")
    s = s.replace(test_marker, tests + test_marker, 1)

needle = "        assert_eq!(report.mode, IncrementalMode::FullRebuild);\n    }\n\n    #[test]\n    fn structural_change_still_falls_back_to_full_rebuild()"
if needle in s:
    s = s.replace(
        needle,
        "        assert_eq!(report.mode, IncrementalMode::FullRebuild);\n        assert!(report.styles_rebuilt);\n    }\n\n    #[test]\n    fn structural_change_still_falls_back_to_full_rebuild()",
        1,
    )

path.write_text(s)
