use rarog_css::StyleSet;
use rarog_dom::{Document, ElementData, NodeKind};
use rarog_layout::{LayoutNode, LayoutNodeKind, build_layout_tree};
use rarog_types::Color;
use std::collections::BTreeMap;

fn find_text(node: &LayoutNode) -> Option<&LayoutNode> {
    if matches!(node.kind, LayoutNodeKind::Text(_)) {
        return Some(node);
    }
    node.children.iter().find_map(find_text)
}

#[test]
fn text_layout_nodes_carry_inherited_color() {
    let mut document = Document::new();
    let mut attributes = BTreeMap::new();
    attributes.insert("style".into(), "color:#112233".into());
    let parent = document
        .append_new(
            document.root(),
            NodeKind::Element(ElementData::html("div").with_attributes(attributes)),
        )
        .unwrap();
    document
        .append_new(parent, NodeKind::Text("Rarog".into()))
        .unwrap();

    let styles = StyleSet::for_document(&document);
    let tree = build_layout_tree(&document, &styles);
    let text = find_text(&tree.root).expect("text node is in layout tree");
    assert_eq!(text.style.color, Color::rgb(0x11, 0x22, 0x33));
}
