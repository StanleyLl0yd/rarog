from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text(encoding="utf-8")
old = """    fn build_node(&mut self, doc: &Document, node: NodeId) -> Option<LayoutNode> {\n        let (kind, style) = match &doc.node(node).kind {"""
new = """    fn build_node(&mut self, doc: &Document, node: NodeId) -> Option<LayoutNode> {\n        let dom_node = doc.node(node)?;\n        let (kind, style) = match &dom_node.kind {"""
if text.count(old) != 1:
    raise SystemExit("unexpected build_node pattern")
text = text.replace(old, new)
old = "for child in doc.children(node) {"
new = "for child in doc.children(node).unwrap_or(&[]) {"
if text.count(old) != 1:
    raise SystemExit("unexpected children pattern")
path.write_text(text.replace(old, new), encoding="utf-8")
