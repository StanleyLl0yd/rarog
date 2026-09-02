use rarog_css::{StyleSet, VerticalAlign, computed_style};
use rarog_dom::{Document, ElementData, NodeKind};
use std::collections::BTreeMap;

fn styled_element(document: &mut Document, style: &str) -> rarog_dom::NodeId {
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
fn vertical_align_keywords_reach_computed_style() {
    let mut document = Document::new();
    let baseline = styled_element(&mut document, "display:inline");
    let top = styled_element(&mut document, "display:inline;vertical-align:top");
    let bottom = styled_element(&mut document, "display:inline;vertical-align:bottom");
    let styles = StyleSet::for_document(&document);

    assert_eq!(
        computed_style(&document, baseline, &styles).vertical_align,
        VerticalAlign::Baseline
    );
    assert_eq!(
        computed_style(&document, top, &styles).vertical_align,
        VerticalAlign::Top
    );
    assert_eq!(
        computed_style(&document, bottom, &styles).vertical_align,
        VerticalAlign::Bottom
    );
}

#[test]
fn vertical_align_css_wide_reset_uses_baseline_initial_value() {
    let mut document = Document::new();
    let node = styled_element(
        &mut document,
        "display:inline;vertical-align:bottom;vertical-align:initial",
    );
    let styles = StyleSet::for_document(&document);

    assert_eq!(
        computed_style(&document, node, &styles).vertical_align,
        VerticalAlign::Baseline
    );
}
