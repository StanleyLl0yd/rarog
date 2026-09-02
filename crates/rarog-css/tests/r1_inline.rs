use rarog_css::{StyleSet, computed_style};
use rarog_dom::{Document, ElementData, NodeKind};
use std::collections::BTreeMap;

fn element(document: &mut Document, style: &str) -> rarog_dom::NodeId {
    let mut attributes = BTreeMap::new();
    attributes.insert("style".into(), style.into());
    document
        .append_new(
            document.root(),
            NodeKind::Element(ElementData::html("span").with_attributes(attributes)),
        )
        .unwrap()
}

#[test]
fn display_inline_reaches_computed_style() {
    let mut document = Document::new();
    let node = element(&mut document, "display:inline");
    let styles = StyleSet::for_document(&document);
    let style = computed_style(&document, node, &styles);

    assert!(style.display_inline);
    assert!(!style.display_none);
    assert!(!style.establishes_bfc);
}

#[test]
fn later_block_value_leaves_inline_mode() {
    let mut document = Document::new();
    let node = element(&mut document, "display:inline;display:block");
    let styles = StyleSet::for_document(&document);
    let style = computed_style(&document, node, &styles);

    assert!(!style.display_inline);
    assert!(!style.display_none);
    assert!(!style.establishes_bfc);
}
