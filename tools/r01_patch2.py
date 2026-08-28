from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} occurrence(s), found {found}: {old!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


css = "crates/rarog-css/src/lib.rs"
replace(
    css,
    """    let Some(node) = document.node(node) else {\n        return ComputedStyle::default();\n    };\n    let NodeKind::Element(element) = &node.kind else {""",
    """    let Some(dom_node) = document.node(node) else {\n        return ComputedStyle::default();\n    };\n    let NodeKind::Element(element) = &dom_node.kind else {""",
)

html = "crates/rarog-html/src/lib.rs"
replace(
    html,
    "let html = output.document.children(output.document.root())[0];",
    "let html = output.document.children(output.document.root()).unwrap()[0];",
)
replace(
    html,
    "let NodeKind::Element(element) = &output.document.node(html).kind else {",
    "let NodeKind::Element(element) = &output.document.node(html).unwrap().kind else {",
)
