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

start = s.find("    #[test]\n    fn character_data_and_geometry_style_update_remain_full_rebuild() {")
end = s.find("    #[test]\n    fn style_element_character_data_revalidates_retained_layout() {", start)
if start < 0 or end < 0:
    raise SystemExit("mixed geometry unit-test boundaries missing")
block = s[start:end]
block = block.replace(
    "fn character_data_and_geometry_style_update_remain_full_rebuild()",
    "fn character_data_and_geometry_style_update_share_retained_flow_relayout()",
    1,
)
block = block.replace(
    'session.update().expect("mixed fallback update succeeds")',
    'session.update().expect("mixed retained-flow update succeeds")',
    1,
)
block = block.replace(
    "        assert_eq!(report.mode, IncrementalMode::FullRebuild);\n",
    "        assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n        assert!(report.retained_display_list);\n",
    1,
)
s = s[:start] + block + s[end:]
engine.write_text(s)

inline = Path("crates/rarog-engine/tests/r1_inline.rs")
s = inline.read_text()
if "fn inline_geometry_change_uses_correct_full_rebuild()" not in s:
    raise SystemExit("inline geometry test name missing")
s = s.replace(
    "fn inline_geometry_change_uses_correct_full_rebuild()",
    "fn inline_geometry_change_uses_retained_flow_relayout()",
    1,
)
start = s.find("fn inline_geometry_change_uses_retained_flow_relayout()")
end = s.find("#[test]\nfn inline_paint_only_change_keeps_retained_path()", start)
block = s[start:end]
block = block.replace(
    "    assert_eq!(report.mode, IncrementalMode::FullRebuild);\n",
    "    assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n    assert!(report.retained_display_list);\n",
    1,
)
s = s[:start] + block + s[end:]
inline.write_text(s)

fragmentation = Path("crates/rarog-engine/tests/r1_inline_fragmentation.rs")
s = fragmentation.read_text()
if "fn fragmented_inline_style_change_rebuilds_all_fragments()" not in s:
    raise SystemExit("fragmented inline test name missing")
s = s.replace(
    "fn fragmented_inline_style_change_rebuilds_all_fragments()",
    "fn fragmented_inline_style_change_uses_retained_flow_relayout()",
    1,
)
start = s.find("fn fragmented_inline_style_change_uses_retained_flow_relayout()")
if start < 0:
    raise SystemExit("fragmented inline updated test missing")
block = s[start:]
block = block.replace(
    "    assert_eq!(report.mode, IncrementalMode::FullRebuild);\n",
    "    assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n    assert!(report.retained_display_list);\n",
    1,
)
s = s[:start] + block
fragmentation.write_text(s)
