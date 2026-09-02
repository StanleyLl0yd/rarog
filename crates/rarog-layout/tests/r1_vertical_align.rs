use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{fragment_for_dom, fragments_for_dom, layout_document};
use rarog_types::Size;
use std::collections::BTreeMap;

fn append_box(document: &mut Document, parent: NodeId, style: &str) -> NodeId {
    let mut attributes = BTreeMap::new();
    if !style.is_empty() {
        attributes.insert("style".into(), style.into());
    }
    document
        .append_new(
            parent,
            NodeKind::Element(ElementData::html("span").with_attributes(attributes)),
        )
        .unwrap()
}

fn append_text(document: &mut Document, parent: NodeId, text: &str) -> NodeId {
    document
        .append_new(parent, NodeKind::Text(text.into()))
        .unwrap()
}

fn host(document: &mut Document) -> NodeId {
    let root = document.root();
    append_box(document, root, "")
}

fn layout(document: &Document) -> rarog_layout::LayoutOutput {
    layout_document(
        document,
        Size {
            width: 120.0,
            height: 200.0,
        },
    )
}

#[test]
fn default_atomic_inline_baseline_aligns_with_text_baseline() {
    let mut document = Document::new();
    let host = host(&mut document);
    let text = append_text(&mut document, host, "a");
    let inline = append_box(&mut document, host, "display:inline;width:20px;height:10px");
    let block = append_box(&mut document, host, "height:10px");

    let output = layout(&document);
    let text = fragments_for_dom(&output.fragments, text);
    let inline = fragment_for_dom(&output.fragments, inline).unwrap();
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(text[0].boxes.content_box.origin.y, 0.0);
    assert_eq!(inline.boxes.margin_box.origin.y, 4.0);
    assert_eq!(block.boxes.border_box.origin.y, 18.0);
}

#[test]
fn tall_baseline_box_raises_text_and_expands_the_line() {
    let mut document = Document::new();
    let host = host(&mut document);
    let text = append_text(&mut document, host, "a");
    let inline = append_box(&mut document, host, "display:inline;width:20px;height:30px");
    let block = append_box(&mut document, host, "height:10px");

    let output = layout(&document);
    let text = fragments_for_dom(&output.fragments, text);
    let inline = fragment_for_dom(&output.fragments, inline).unwrap();
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(inline.boxes.margin_box.origin.y, 0.0);
    assert_eq!(text[0].boxes.content_box.origin.y, 16.0);
    assert_eq!(block.boxes.border_box.origin.y, 34.0);
}

#[test]
fn vertical_align_top_uses_the_line_top() {
    let mut document = Document::new();
    let host = host(&mut document);
    let text = append_text(&mut document, host, "a");
    let inline = append_box(
        &mut document,
        host,
        "display:inline;width:20px;height:30px;vertical-align:top",
    );
    let block = append_box(&mut document, host, "height:10px");

    let output = layout(&document);
    let text = fragments_for_dom(&output.fragments, text);
    let inline = fragment_for_dom(&output.fragments, inline).unwrap();
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(text[0].boxes.content_box.origin.y, 0.0);
    assert_eq!(inline.boxes.margin_box.origin.y, 0.0);
    assert_eq!(block.boxes.border_box.origin.y, 30.0);
}

#[test]
fn vertical_align_bottom_uses_the_line_bottom() {
    let mut document = Document::new();
    let host = host(&mut document);
    append_text(&mut document, host, "a");
    let inline = append_box(
        &mut document,
        host,
        "display:inline;width:20px;height:10px;vertical-align:bottom",
    );
    let block = append_box(&mut document, host, "height:10px");

    let output = layout(&document);
    let inline = fragment_for_dom(&output.fragments, inline).unwrap();
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(inline.boxes.margin_box.origin.y, 8.0);
    assert_eq!(block.boxes.border_box.origin.y, 18.0);
}
