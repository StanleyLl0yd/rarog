from pathlib import Path

engine = Path("crates/rarog-engine/src/lib.rs")
s = engine.read_text()

old = '''                    if fragments_for_dom(&self.layout.fragments, node).len() > 1 {
                        requires_full_rebuild = true;
                        break;
                    }
'''
new = '''                    if fragments_for_dom(&self.layout.fragments, node).len() > 1 {
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
    raise SystemExit("multi-fragment fallback marker missing")
s = s.replace(old, new, 1)

old = '''                    if layout_changed
                        && (!text_relayout_nodes.is_empty()
                            || old_style.display_inline
                            || new_style.display_inline)
                    {
                        requires_full_rebuild = true;
                        break;
                    }
'''
new = '''                    if layout_changed
                        && (!text_relayout_nodes.is_empty()
                            || old_style.display_inline
                            || new_style.display_inline)
                    {
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
    raise SystemExit("mixed/inline geometry fallback marker missing")
s = s.replace(old, new, 1)

old = '''    #[test]
    fn character_data_and_geometry_style_update_remain_full_rebuild() {
        let source = "<div id=\"target\" style=\"width:48px;background:#112233\">one</div>";
        let expected_source =
            "<div id=\"target\" style=\"width:72px;background:#778899\">one two three four</div>";
        let mut session = session(source, deterministic_options());
        let target = element_with_id(session.document(), "target");
        let text = *session
            .document()
            .children(target)
            .and_then(|children| children.first())
            .expect("target contains a text node");

        session
            .document_mut()
            .set_text(text, "one two three four")
            .unwrap();
        session
            .document_mut()
            .set_attribute(target, "style", "width:72px;background:#778899")
            .unwrap();

        let report = session.update().expect("mixed fallback update succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }
'''
new = '''    #[test]
    fn character_data_and_geometry_style_update_share_retained_flow_relayout() {
        let source = "<div id=\"target\" style=\"width:48px;background:#112233\">one</div>";
        let expected_source =
            "<div id=\"target\" style=\"width:72px;background:#778899\">one two three four</div>";
        let mut session = session(source, deterministic_options());
        let target = element_with_id(session.document(), "target");
        let text = *session
            .document()
            .children(target)
            .and_then(|children| children.first())
            .expect("target contains a text node");

        session
            .document_mut()
            .set_text(text, "one two three four")
            .unwrap();
        session
            .document_mut()
            .set_attribute(target, "style", "width:72px;background:#778899")
            .unwrap();

        let report = session.update().expect("mixed retained-flow update succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert!(report.retained_display_list);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }
'''
if old not in s:
    raise SystemExit("mixed geometry unit test marker missing")
s = s.replace(old, new, 1)
engine.write_text(s)

inline = Path("crates/rarog-engine/tests/r1_inline.rs")
s = inline.read_text()
s = s.replace(
    "fn inline_geometry_change_uses_correct_full_rebuild() {",
    "fn inline_geometry_change_uses_retained_flow_relayout() {",
    1,
)
s = s.replace(
    "    assert_eq!(report.mode, IncrementalMode::FullRebuild);\n    assert_eq!(\n        session.framebuffer().stable_hash64(),\n        fresh.framebuffer.stable_hash64()\n    );\n}\n\n#[test]\nfn inline_paint_only_change_keeps_retained_path() {",
    "    assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n    assert!(report.retained_display_list);\n    assert_eq!(\n        session.framebuffer().stable_hash64(),\n        fresh.framebuffer.stable_hash64()\n    );\n}\n\n#[test]\nfn inline_paint_only_change_keeps_retained_path() {",
    1,
)
inline.write_text(s)

fragmentation = Path("crates/rarog-engine/tests/r1_inline_fragmentation.rs")
s = fragmentation.read_text()
s = s.replace(
    "fn fragmented_inline_style_change_rebuilds_all_fragments() {",
    "fn fragmented_inline_style_change_uses_retained_flow_relayout() {",
    1,
)
old_assert = '''    assert_eq!(report.mode, IncrementalMode::FullRebuild);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
'''
new_assert = '''    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert!(report.retained_display_list);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
'''
if old_assert not in s:
    raise SystemExit("fragmented inline expectation marker missing")
s = s.replace(old_assert, new_assert, 1)
fragmentation.write_text(s)
