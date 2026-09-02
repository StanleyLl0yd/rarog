use rarog_css::{StyleSet, computed_style};
use rarog_dom::{Document, ElementData, NodeKind};
use std::collections::BTreeMap;

fn styled_element(document: &mut Document, style: &str) -> rarog_dom::NodeId {
    let mut attributes = BTreeMap::new();
    attributes.insert("style".into(), style.into());
    document
        .append_new(
            document.root(),
            NodeKind::Element(ElementData::html("div").with_attributes(attributes)),
        )
        .unwrap()
}

#[test]
fn sizing_keywords_and_constraints_reach_computed_style() {
    let mut document = Document::new();
    let node = styled_element(
        &mut document,
        "width:auto;height:auto;min-width:80px;max-width:120px;min-height:10px;max-height:40px",
    );
    let styles = StyleSet::for_document(&document);
    let style = computed_style(&document, node, &styles);

    assert_eq!(style.width, None);
    assert_eq!(style.height, None);
    assert_eq!(style.min_width, Some(80.0));
    assert_eq!(style.max_width, Some(120.0));
    assert_eq!(style.min_height, Some(10.0));
    assert_eq!(style.max_height, Some(40.0));
}

#[test]
fn later_auto_none_and_flow_root_override_prior_values() {
    let mut document = Document::new();
    let node = styled_element(
        &mut document,
        "width:90px;width:auto;min-width:20px;min-width:auto;max-width:100px;max-width:none;display:flow-root",
    );
    let styles = StyleSet::for_document(&document);
    let style = computed_style(&document, node, &styles);

    assert_eq!(style.width, None);
    assert_eq!(style.min_width, None);
    assert_eq!(style.max_width, None);
    assert!(!style.display_none);
    assert!(style.establishes_bfc);
}
