from pathlib import Path

# Keep legacy regressions aligned with the retained stylesheet-source path.
path = Path("crates/rarog-engine/src/lib.rs")
s = path.read_text()
start = s.find("    #[test]\n    fn style_element_character_data_still_requires_full_rebuild() {")
if start >= 0:
    end = s.find("    #[test]", start + 12)
    if end < 0:
        raise SystemExit("legacy stylesheet unit test end missing")
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
elif "fn style_element_character_data_revalidates_retained_layout()" not in s:
    raise SystemExit("legacy stylesheet unit test missing")
path.write_text(s)

correctness = Path("crates/rarog-engine/tests/r01_correctness.rs")
c = correctness.read_text()
old = '''    assert_eq!(
        session.update().expect("stylesheet update succeeds").mode,
        IncrementalMode::FullRebuild
    );
    assert_matches_fresh(&session, expected);
'''
new = '''    let report = session.update().expect("stylesheet update succeeds");
    assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
    assert!(report.retained_display_list);
    assert!(report.styles_rebuilt);
    assert_matches_fresh(&session, expected);
'''
if old in c:
    c = c.replace(old, new, 1)
elif 'assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);' not in c:
    raise SystemExit("legacy R0.1 stylesheet expectation missing")
correctness.write_text(c)
