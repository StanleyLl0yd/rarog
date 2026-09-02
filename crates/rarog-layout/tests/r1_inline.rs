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
            NodeKind::Element(ElementData::html("span").with_attributes(attributes)),
        )
        .unwrap()
}

fn layout(document: &Document, width: f32) -> rarog_layout::LayoutOutput {
    layout_document(
        document,
        Size {
            width,
            height: 200.0,
        },
    )
}

#[test]
fn atomic_inline_boxes_pack_horizontally_and_wrap() {
    let mut document = Document::new();
    let root = document.root();
    let first = append_box(&mut document, root, "display:inline;width:40px;height:10px");
    let second = append_box(&mut document, root, "display:inline;width:40px;height:10px");
    let third = append_box(&mut document, root, "display:inline;width:40px;height:10px");

    let output = layout(&document, 100.0);
    let first = fragment_for_dom(&output.fragments, first).unwrap();
    let second = fragment_for_dom(&output.fragments, second).unwrap();
    let third = fragment_for_dom(&output.fragments, third).unwrap();

    assert_eq!(first.boxes.border_box.origin.x, 0.0);
    assert_eq!(first.boxes.border_box.origin.y, 0.0);
    assert_eq!(second.boxes.border_box.origin.x, 40.0);
    assert_eq!(second.boxes.border_box.origin.y, 0.0);
    assert_eq!(third.boxes.border_box.origin.x, 0.0);
    assert_eq!(third.boxes.border_box.origin.y, 10.0);
}

#[test]
fn inline_edges_contribute_to_line_packing() {
    let mut document = Document::new();
    let root = document.root();
    let first = append_box(
        &mut document,
        root,
        "display:inline;width:20px;height:10px;margin-left:5px;margin-right:5px;padding:2px;border-width:1px",
    );
    let second = append_box(&mut document, root, "display:inline;width:20px;height:10px");

    let output = layout(&document, 100.0);
    let first = fragment_for_dom(&output.fragments, first).unwrap();
    let second = fragment_for_dom(&output.fragments, second).unwrap();

    assert_eq!(first.boxes.margin_box.size.width, 36.0);
    assert_eq!(first.boxes.border_box.origin.x, 5.0);
    assert_eq!(second.boxes.border_box.origin.x, 36.0);
}

#[test]
fn block_content_flushes_the_active_inline_line() {
    let mut document = Document::new();
    let root = document.root();
    let first = append_box(&mut document, root, "display:inline;width:40px;height:10px");
    let block = append_box(&mut document, root, "display:block;height:20px");
    let last = append_box(&mut document, root, "display:inline;width:30px;height:10px");

    let output = layout(&document, 100.0);
    let first = fragment_for_dom(&output.fragments, first).unwrap();
    let block = fragment_for_dom(&output.fragments, block).unwrap();
    let last = fragment_for_dom(&output.fragments, last).unwrap();

    assert_eq!(first.boxes.border_box.origin.y, 0.0);
    assert_eq!(block.boxes.border_box.origin.y, 10.0);
    assert_eq!(last.boxes.border_box.origin.y, 30.0);
}

#[test]
fn tallest_inline_box_sets_the_line_advance() {
    let mut document = Document::new();
    let root = document.root();
    append_box(&mut document, root, "display:inline;width:30px;height:10px");
    append_box(&mut document, root, "display:inline;width:30px;height:25px");
    let block = append_box(&mut document, root, "height:10px");

    let output = layout(&document, 100.0);
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(block.boxes.border_box.origin.y, 25.0);
}
