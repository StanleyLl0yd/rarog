use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{fragment_for_dom, layout_document};
use rarog_types::Size;
use std::collections::BTreeMap;

fn append_box(document: &mut Document, parent: NodeId, tag: &str, style: &str) -> NodeId {
    let mut attributes = BTreeMap::new();
    if !style.is_empty() {
        attributes.insert("style".into(), style.into());
    }
    document
        .append_new(
            parent,
            NodeKind::Element(ElementData::html(tag).with_attributes(attributes)),
        )
        .unwrap()
}

fn flow_host(document: &mut Document) -> NodeId {
    let root = document.root();
    append_box(document, root, "main", "")
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
fn parent_top_margin_collapses_with_first_block_child() {
    let mut document = Document::new();
    let host = flow_host(&mut document);
    let parent = append_box(&mut document, host, "div", "margin-top:10px");
    let child = append_box(&mut document, parent, "div", "height:20px;margin-top:30px");

    let output = layout(&document);
    let parent = fragment_for_dom(&output.fragments, parent).unwrap();
    let child = fragment_for_dom(&output.fragments, child).unwrap();

    assert_eq!(parent.boxes.border_box.origin.y, 30.0);
    assert_eq!(
        child.boxes.border_box.origin.y,
        parent.boxes.content_box.origin.y
    );
}

#[test]
fn parent_padding_blocks_top_margin_collapse() {
    let mut document = Document::new();
    let host = flow_host(&mut document);
    let parent = append_box(
        &mut document,
        host,
        "div",
        "margin-top:10px;padding-top:5px",
    );
    let child = append_box(&mut document, parent, "div", "height:20px;margin-top:30px");

    let output = layout(&document);
    let parent = fragment_for_dom(&output.fragments, parent).unwrap();
    let child = fragment_for_dom(&output.fragments, child).unwrap();

    assert_eq!(parent.boxes.border_box.origin.y, 10.0);
    assert_eq!(child.boxes.border_box.origin.y, 45.0);
}

#[test]
fn parent_bottom_margin_collapses_with_last_child_and_following_sibling() {
    let mut document = Document::new();
    let host = flow_host(&mut document);
    let parent = append_box(&mut document, host, "div", "margin-bottom:10px");
    let child = append_box(
        &mut document,
        parent,
        "div",
        "height:20px;margin-bottom:30px",
    );
    let next = append_box(&mut document, host, "div", "height:10px;margin-top:20px");

    let output = layout(&document);
    let parent = fragment_for_dom(&output.fragments, parent).unwrap();
    let child = fragment_for_dom(&output.fragments, child).unwrap();
    let next = fragment_for_dom(&output.fragments, next).unwrap();

    assert_eq!(
        child.boxes.border_box.origin.y,
        parent.boxes.content_box.origin.y
    );
    assert_eq!(parent.boxes.border_box.size.height, 20.0);
    assert_eq!(next.boxes.border_box.origin.y, 50.0);
}

#[test]
fn empty_block_margins_collapse_through_to_surrounding_siblings() {
    let mut document = Document::new();
    let host = flow_host(&mut document);
    append_box(&mut document, host, "div", "height:20px;margin-bottom:10px");
    append_box(
        &mut document,
        host,
        "div",
        "margin-top:30px;margin-bottom:40px",
    );
    let next = append_box(&mut document, host, "div", "height:10px;margin-top:20px");

    let output = layout(&document);
    let next = fragment_for_dom(&output.fragments, next).unwrap();

    assert_eq!(next.boxes.border_box.origin.y, 60.0);
}

#[test]
fn padding_prevents_empty_block_margin_pass_through() {
    let mut document = Document::new();
    let host = flow_host(&mut document);
    append_box(&mut document, host, "div", "height:20px;margin-bottom:10px");
    append_box(
        &mut document,
        host,
        "div",
        "margin-top:30px;margin-bottom:40px;padding-top:1px;padding-bottom:1px",
    );
    let next = append_box(&mut document, host, "div", "height:10px;margin-top:20px");

    let output = layout(&document);
    let next = fragment_for_dom(&output.fragments, next).unwrap();

    assert_eq!(next.boxes.border_box.origin.y, 92.0);
}

#[test]
fn document_element_is_a_margin_collapse_boundary() {
    let mut document = Document::new();
    let root = document.root();
    let html = append_box(&mut document, root, "html", "");
    let body = append_box(&mut document, html, "body", "height:10px;margin:8px");

    let output = layout(&document);
    let html = fragment_for_dom(&output.fragments, html).unwrap();
    let body = fragment_for_dom(&output.fragments, body).unwrap();

    assert_eq!(html.boxes.border_box.origin.y, 0.0);
    assert_eq!(body.boxes.border_box.origin.y, 8.0);
}
