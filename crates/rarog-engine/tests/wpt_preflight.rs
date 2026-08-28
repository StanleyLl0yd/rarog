const R1_WPT_FOCUS: &str = include_str!("../../../wpt/r1-focus.txt");

#[test]
fn r1_wpt_focus_manifest_is_nonempty_and_directory_scoped() {
    let entries = R1_WPT_FOCUS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();

    assert!(!entries.is_empty(), "R1 WPT focus manifest must not be empty");
    assert!(
        entries.iter().all(|entry| entry.ends_with('/')),
        "R1 WPT preflight entries must be directory-scoped"
    );
    assert!(
        entries.iter().any(|entry| entry.starts_with("html/")),
        "R1 WPT focus must include HTML"
    );
    assert!(
        entries.iter().any(|entry| entry.starts_with("css/")),
        "R1 WPT focus must include CSS"
    );
}
