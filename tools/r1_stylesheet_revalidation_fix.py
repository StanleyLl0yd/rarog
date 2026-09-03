from pathlib import Path

# Keep the legacy regression aligned with the retained stylesheet-source path.
path = Path("crates/rarog-engine/src/lib.rs")
s = path.read_text()
start = s.find("    #[test]\n    fn style_element_character_data_still_requires_full_rebuild() {")
if start < 0:
    if "fn style_element_character_data_revalidates_retained_layout()" in s:
        raise SystemExit(0)
    raise SystemExit("legacy stylesheet test missing")
end = s.find("    #[test]", start + 12)
if end < 0:
    raise SystemExit("legacy stylesheet test end missing")
block = s[start:end]
block = block.replace(
    "fn style_element_character_data_still_requires_full_rebuild()",
    "fn style_element_character_data_revalidates_retained_layout()",
    1,
)
block = block.replace(
    "assert_eq!(report.mode, IncrementalMode::FullRebuild);",
    "assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);\n        assert!(report.retained_display_list);",
    1,
)
s = s[:start] + block + s[end:]
path.write_text(s)
