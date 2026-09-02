use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{fragment_for_dom, layout_document};
use rarog_types::Size;
use std::collections::BTreeMap;

fn append_box(document: &mut Document, parent: NodeId, style: &str) -> NodeId {
    let mut attributes = BTreeMap::new();
    attributes.insert("style".into(), style.into());
    document
        .append_new(
            parent,
            NodeKind::Element(ElementData::html("div").with_attributes(attributes)),
        )
        .unwrap()
}

fn layout(document: &Document) -> rarog_layout::LayoutOutput {
    layout_document(
        document,
        Size {
            width: 240.0,
            height: 240.0,
        },
    )
}

#[test]
fn adjoining_positive_block_margins_collapse_to_the_larger_margin() {
    let mut document = Document::new();
    let root = document.root();
    let first = append_box(&mut document, root, "height:20px;margin-bottom:20px");
    let second = append_box(&mut document, root, "height:20px;margin-top:30px");

    let output = layout(&document);
    let first = fragment_for_dom(&output.fragments, first).unwrap();
    let second = fragment_for_dom(&output.fragments, second).unwrap();

    let first_bottom = first.boxes.border_box.origin.y + first.boxes.border_box.size.height;
    assert_eq!(first_bottom, 20.0);
    assert_eq!(second.boxes.border_box.origin.y, 50.0);
}

#[test]
fn adjoining_positive_and_negative_margins_sum_the_extremes() {
    let mut document = Document::new();
    let root = document.root();
    append_box(&mut document, root, "height:20px;margin-bottom:20px");
    let second = append_box(&mut document, root, "height:20px;margin-top:-8px");

    let output = layout(&document);
    let second = fragment_for_dom(&output.fragments, second).unwrap();

    assert_eq!(second.boxes.border_box.origin.y, 32.0);
}

#[test]
fn adjoining_negative_margins_collapse_to_the_most_negative_margin() {
    let mut document = Document::new();
    let root = document.root();
    append_box(&mut document, root, "height:20px;margin-bottom:-10px");
    let second = append_box(&mut document, root, "height:20px;margin-top:-20px");

    let output = layout(&document);
    let second = fragment_for_dom(&output.fragments, second).unwrap();

    assert_eq!(second.boxes.border_box.origin.y, 0.0);
}

#[test]
fn text_content_interrupts_block_margin_collapsing() {
    let mut document = Document::new();
    let root = document.root();
    let first = append_box(&mut document, root, "height:20px;margin-bottom:20px");
    let text = document
        .append_new(root, NodeKind::Text("x".into()))
        .unwrap();
    let second = append_box(&mut document, root, "height:20px;margin-top:30px");

    let output = layout(&document);
    let first = fragment_for_dom(&output.fragments, first).unwrap();
    let text = fragment_for_dom(&output.fragments, text).unwrap();
    let second = fragment_for_dom(&output.fragments, second).unwrap();

    let first_bottom = first.boxes.border_box.origin.y + first.boxes.border_box.size.height;
    assert_eq!(text.boxes.content_box.origin.y, first_bottom + 20.0);
    assert_eq!(
        second.boxes.border_box.origin.y,
        text.boxes.content_box.origin.y + text.boxes.content_box.size.height + 30.0
    );
}
