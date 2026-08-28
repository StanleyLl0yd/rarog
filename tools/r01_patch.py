from pathlib import Path


def replace(path: str, old: str, new: str, count: int | None = None) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    expected = count if count is not None else 1
    if found != expected:
        raise SystemExit(f"{path}: expected {expected} occurrence(s), found {found}: {old!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


css = "crates/rarog-css/src/lib.rs"
replace(
    css,
    """    pub fn matches(&self, document: &Document, node: NodeId) -> bool {\n        let NodeKind::Element(element) = &document.node(node).kind else {\n            return false;\n        };""",
    """    pub fn matches(&self, document: &Document, node: NodeId) -> bool {\n        let Some(node) = document.node(node) else {\n            return false;\n        };\n        let NodeKind::Element(element) = &node.kind else {\n            return false;\n        };""",
)
replace(
    css,
    """pub fn style_sharing_key(document: &Document, node: NodeId) -> Option<StyleSharingKey> {\n    let NodeKind::Element(element) = &document.node(node).kind else {\n        return None;\n    };""",
    """pub fn style_sharing_key(document: &Document, node: NodeId) -> Option<StyleSharingKey> {\n    let node = document.node(node)?;\n    let NodeKind::Element(element) = &node.kind else {\n        return None;\n    };""",
)
replace(
    css,
    """pub fn computed_style(document: &Document, node: NodeId, styles: &StyleSet) -> ComputedStyle {\n    let NodeKind::Element(element) = &document.node(node).kind else {\n        return ComputedStyle::default();\n    };""",
    """pub fn computed_style(document: &Document, node: NodeId, styles: &StyleSet) -> ComputedStyle {\n    let Some(node) = document.node(node) else {\n        return ComputedStyle::default();\n    };\n    let NodeKind::Element(element) = &node.kind else {\n        return ComputedStyle::default();\n    };""",
)
replace(
    css,
    """fn collect_style_elements(document: &Document, node: NodeId, output: &mut Vec<String>) {\n    if let NodeKind::Element(element) = &document.node(node).kind {""",
    """fn collect_style_elements(document: &Document, node: NodeId, output: &mut Vec<String>) {\n    let Some(current) = document.node(node) else {\n        return;\n    };\n    if let NodeKind::Element(element) = &current.kind {""",
)
replace(css, "for child in document.children(node) {", "for child in document.children(node).unwrap_or(&[]) {", 4)
replace(
    css,
    """        match &document.node(*child).kind {\n            NodeKind::Text(text) => {""",
    """        match document.node(*child).map(|node| &node.kind) {\n            Some(NodeKind::Text(text)) => {""",
)
replace(
    css,
    """        for record in document.mutation_records_since(generation) {\n            match &record.kind {""",
    """        let records = match document.mutation_records_since(generation) {\n            Ok(records) => records,\n            Err(_) => return Self::for_stylesheet_change(document),\n        };\n\n        for record in records {\n            match &record.kind {""",
)
replace(css, "let parent = document.node(*node).parent;", "let parent = document.node(*node).and_then(|node| node.parent);", 2)
replace(css, "node = document.node(current).parent;", "node = document.node(current).and_then(|node| node.parent);")
replace(
    css,
    """                    if let Some(parent) = document.node(node).parent {""",
    """                    if let Some(parent) = document.node(node).and_then(|node| node.parent) {""",
)
replace(
    css,
    """        let children = document.children(parent);\n        let Some(position) = children.iter().position(|child| *child == node) else {""",
    """        let Some(children) = document.children(parent) else {\n            return;\n        };\n        let Some(position) = children.iter().position(|child| *child == node) else {""",
)
replace(css, "for child in document.children(parent) {", "for child in document.children(parent).unwrap_or(&[]) {")

html = "crates/rarog-html/src/lib.rs"
replace(
    html,
    "if !self_closing && !matches_void(document.node(id)) {",
    "if !self_closing && document.node(id).is_some_and(|node| !matches_void(node)) {",
)

dom = "crates/rarog-dom/src/lib.rs"
replace(
    dom,
    "doc.mutation_records_since(consumed - 1).unwrap_err(),",
    "doc.mutation_records_since(consumed - 1).err().unwrap(),",
)
