from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
s = path.read_text()

old_unchanged = '''        } else if style_updates.is_empty() && text_relayout_nodes.is_empty() {
'''
new_unchanged = '''        } else if style_updates.is_empty()
            && text_relayout_nodes.is_empty()
            && structural_relayout_nodes.is_empty()
        {
'''
if old_unchanged in s:
    s = s.replace(old_unchanged, new_unchanged, 1)
elif "&& structural_relayout_nodes.is_empty()" not in s:
    raise SystemExit("unchanged classification marker missing")

old_ordinary = '''        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert!(!report.styles_rebuilt);
        assert_eq!(session.styles(), &expected.styles);
'''
new_ordinary = '''        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert!(report.retained_display_list);
        assert!(!report.styles_rebuilt);
        assert_eq!(session.styles(), &expected.styles);
'''
if old_ordinary in s:
    s = s.replace(old_ordinary, new_ordinary, 1)
elif "ordinary_structural_change_reuses_existing_style_set" in s and "assert!(report.retained_display_list);" not in s:
    raise SystemExit("ordinary structural expectation marker missing")

old_fallback = '''    #[test]
    fn structural_change_still_falls_back_to_full_rebuild() {
        let mut session = session(
            "<div style=\"width:80px;height:20px\">Rarog</div>",
            deterministic_options(),
        );
        let parent = first_element(session.document());
        session
            .document_mut()
            .append_new(parent, NodeKind::Text("!".into()))
            .unwrap();

        let report = session.update().expect("incremental update succeeds");
        assert_eq!(report.mode, IncrementalMode::FullRebuild);
    }
'''
new_fallback = '''    #[test]
    fn reparent_still_falls_back_to_full_rebuild() {
        let source = "<div id=\"from\"><span id=\"child\">Rarog</span></div><div id=\"to\"></div>";
        let expected_source = "<div id=\"from\"></div><div id=\"to\"><span id=\"child\">Rarog</span></div>";
        let mut session = session(source, deterministic_options());
        let child = element_with_id(session.document(), "child");
        let destination = element_with_id(session.document(), "to");

        session
            .document_mut()
            .append_child(destination, child)
            .unwrap();

        let report = session.update().expect("reparent fallback succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
    }
'''
if old_fallback in s:
    s = s.replace(old_fallback, new_fallback, 1)
elif "fn reparent_still_falls_back_to_full_rebuild()" not in s:
    raise SystemExit("structural fallback marker missing")

path.write_text(s)
