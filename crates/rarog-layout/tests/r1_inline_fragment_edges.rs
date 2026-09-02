use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{fragments_for_dom, layout_document};
use rarog_types::Size;
use std::collections::BTreeMap;

fn append_element(document: &mut Document, parent: NodeId, style: &str) -> NodeId {
    let mut attributes = BTreeMap::new();
    attributes.insert("style".into(), style.into());
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

#[test]
fn fragmented_inline_slices_horizontal_edges_across_lines() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let inline = append_element(
        &mut document,
        host,
        "display:inline;margin:0 3px 0 2px;border-width:0 5px 0 4px;border-color:#445566;padding:0 7px 0 6px;background:#112233",
    );
    append_text(&mut document, inline, "ab cd ef");

    let output = layout_document(
        &document,
        Size {
            width: 48.0,
            height: 160.0,
        },
    );
    let fragments = fragments_for_dom(&output.fragments, inline);

    assert_eq!(fragments.len(), 3);

    let first = fragments[0];
    assert_eq!(first.boxes.margin_box.origin.x, 0.0);
    assert_eq!(first.boxes.border_box.origin.x, 2.0);
    assert_eq!(first.boxes.padding_box.origin.x, 6.0);
    assert_eq!(first.boxes.content_box.origin.x, 12.0);
    assert_eq!(first.boxes.content_box.size.width, 24.0);
    assert_eq!(first.boxes.margin_box.size.width, 36.0);
    assert_eq!(first.style.border_width.left, 4.0);
    assert_eq!(first.style.border_width.right, 0.0);

    let middle = fragments[1];
    assert_eq!(middle.boxes.margin_box, middle.boxes.content_box);
    assert_eq!(middle.boxes.content_box.size.width, 24.0);
    assert_eq!(middle.style.border_width.left, 0.0);
    assert_eq!(middle.style.border_width.right, 0.0);

    let last = fragments[2];
    assert_eq!(last.boxes.content_box.origin.x, 0.0);
    assert_eq!(last.boxes.content_box.size.width, 16.0);
    assert_eq!(last.boxes.padding_box.size.width, 23.0);
    assert_eq!(last.boxes.border_box.size.width, 28.0);
    assert_eq!(last.boxes.margin_box.size.width, 31.0);
    assert_eq!(last.style.border_width.left, 0.0);
    assert_eq!(last.style.border_width.right, 5.0);
}

#[test]
fn terminal_right_edge_participates_in_the_last_soft_break() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let inline = append_element(
        &mut document,
        host,
        "display:inline;border-width:0 8px 0 0;padding:0 8px 0 0",
    );
    append_text(&mut document, inline, "ab cd ef");

    let output = layout_document(
        &document,
        Size {
            width: 48.0,
            height: 160.0,
        },
    );
    let fragments = fragments_for_dom(&output.fragments, inline);

    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].boxes.content_box.size.width, 48.0);
    assert_eq!(fragments[1].boxes.content_box.size.width, 16.0);
    assert_eq!(fragments[1].boxes.margin_box.size.width, 32.0);
}
