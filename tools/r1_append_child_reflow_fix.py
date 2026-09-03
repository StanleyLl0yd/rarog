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

ordinary_start = s.find("    fn ordinary_structural_change_reuses_existing_style_set() {")
if ordinary_start < 0:
    raise SystemExit("ordinary structural test missing")
ordinary_end = s.find("    #[test]", ordinary_start)
if ordinary_end < 0:
    raise SystemExit("ordinary structural test end missing")
ordinary = s[ordinary_start:ordinary_end]
ordinary = ordinary.replace(
    "assert_eq!(report.mode, IncrementalMode::FullRebuild);",
    "assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n        assert!(report.retained_display_list);",
    1,
)
s = s[:ordinary_start] + ordinary + s[ordinary_end:]

fallback_start = s.find("    #[test]\n    fn structural_change_still_falls_back_to_full_rebuild() {")
if fallback_start < 0:
    if "fn reparent_still_falls_back_to_full_rebuild()" not in s:
        raise SystemExit("structural fallback start missing")
else:
    fallback_end = s.find("    #[test]\n    fn unrelated_attribute_change_does_not_rebuild_render_state() {", fallback_start)
    if fallback_end < 0:
        raise SystemExit("structural fallback end missing")
    replacement = '''    #[test]
    fn reparent_still_falls_back_to_full_rebuild() {
        let source = r#"<div id="from"><span id="child">Rarog</span></div><div id="to"></div>"#;
        let expected_source = r#"<div id="from"></div><div id="to"><span id="child">Rarog</span></div>"#;
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
    s = s[:fallback_start] + replacement + s[fallback_end:]

path.write_text(s)

correctness_path = Path("crates/rarog-engine/tests/r01_correctness.rs")
c = correctness_path.read_text()
start = c.find("#[test]\nfn vertical_flow_and_structural_fallback_are_preserved() {")
end = c.find("#[test]\nfn structural_limits_reject_deep_and_wide_documents_before_recursive_rendering() {", start)
if start < 0 or end < 0:
    if "fn vertical_flow_append_reflow_and_reparent_fallback_are_preserved()" not in c:
        raise SystemExit("R0.1 structural regression marker missing")
else:
    replacement = '''#[test]
fn vertical_flow_append_reflow_and_reparent_fallback_are_preserved() {
    let source = r#"<div id="target" style="height:20px;background:#112233"></div><div style="height:10px;background:#445566"></div>"#;
    let expected = r#"<div id="target" style="height:32px;background:#112233"></div><div style="height:10px;background:#445566"></div>"#;
    let (flow, flow_mode) = update_style(source, "height:32px;background:#112233");
    assert_eq!(flow_mode, IncrementalMode::FlowRelayout);
    assert_matches_fresh(&flow, expected);

    let mut structural = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(structural.document(), "target");
    structural
        .document_mut()
        .append_new(target, NodeKind::Text("!".into()))
        .expect("append mutation succeeds");
    let append_report = structural.update().expect("append update succeeds");
    assert_eq!(append_report.mode, IncrementalMode::FlowRelayout);
    assert!(append_report.retained_display_list);
    assert!(!append_report.styles_rebuilt);
    assert_matches_fresh(
        &structural,
        r#"<div id="target" style="height:20px;background:#112233">!</div><div style="height:10px;background:#445566"></div>"#,
    );

    let reparent_source = r#"<div id="from"><span id="child">Rarog</span></div><div id="to"></div>"#;
    let reparent_expected = r#"<div id="from"></div><div id="to"><span id="child">Rarog</span></div>"#;
    let mut reparent = RenderSession::new(reparent_source, options()).expect("session starts");
    let child = element_with_id(reparent.document(), "child");
    let destination = element_with_id(reparent.document(), "to");
    reparent
        .document_mut()
        .append_child(destination, child)
        .expect("reparent mutation succeeds");
    assert_eq!(
        reparent.update().expect("reparent update succeeds").mode,
        IncrementalMode::FullRebuild
    );
    assert_matches_fresh(&reparent, reparent_expected);
}

'''
    c = c[:start] + replacement + c[end:]
correctness_path.write_text(c)
