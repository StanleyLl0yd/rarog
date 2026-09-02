use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{fragment_for_dom, layout_document};
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
            NodeKind::Element(ElementData::html("div").with_attributes(attributes)),
        )
        .unwrap()
}

fn layout(document: &Document, width: f32) -> rarog_layout::LayoutOutput {
    layout_document(
        document,
        Size {
            width,
            height: 300.0,
        },
    )
}

#[test]
fn auto_width_fills_the_available_content_box() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_box(
        &mut document,
        root,
        "width:auto;margin-left:10px;margin-right:20px;padding:5px;border-width:2px",
    );

    let output = layout(&document, 300.0);
    let fragment = fragment_for_dom(&output.fragments, node).unwrap();

    assert_eq!(fragment.boxes.content_box.size.width, 256.0);
}

#[test]
fn min_and_max_width_clamp_used_block_width_and_min_wins_conflicts() {
    let mut document = Document::new();
    let root = document.root();
    let capped = append_box(
        &mut document,
        root,
        "width:200px;max-width:120px;height:10px",
    );
    let floored = append_box(&mut document, root, "width:50px;min-width:80px;height:10px");
    let conflict = append_box(
        &mut document,
        root,
        "width:50px;min-width:200px;max-width:100px;height:10px",
    );

    let output = layout(&document, 300.0);

    assert_eq!(
        fragment_for_dom(&output.fragments, capped)
            .unwrap()
            .boxes
            .content_box
            .size
            .width,
        120.0
    );
    assert_eq!(
        fragment_for_dom(&output.fragments, floored)
            .unwrap()
            .boxes
            .content_box
            .size
            .width,
        80.0
    );
    assert_eq!(
        fragment_for_dom(&output.fragments, conflict)
            .unwrap()
            .boxes
            .content_box
            .size
            .width,
        200.0
    );
}

#[test]
fn min_and_max_height_clamp_auto_content_height() {
    let mut document = Document::new();
    let root = document.root();
    let min_parent = append_box(&mut document, root, "min-height:50px");
    append_box(&mut document, min_parent, "height:20px");
    let max_parent = append_box(&mut document, root, "max-height:30px");
    append_box(&mut document, max_parent, "height:80px");

    let output = layout(&document, 300.0);

    assert_eq!(
        fragment_for_dom(&output.fragments, min_parent)
            .unwrap()
            .boxes
            .content_box
            .size
            .height,
        50.0
    );
    assert_eq!(
        fragment_for_dom(&output.fragments, max_parent)
            .unwrap()
            .boxes
            .content_box
            .size
            .height,
        30.0
    );
}

#[test]
fn flow_root_establishes_a_parent_child_margin_collapse_boundary() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_box(&mut document, root, "");
    let flow_root = append_box(&mut document, host, "display:flow-root;margin-top:10px");
    let child = append_box(&mut document, flow_root, "height:20px;margin-top:30px");

    let output = layout(&document, 300.0);
    let host = fragment_for_dom(&output.fragments, host).unwrap();
    let flow_root = fragment_for_dom(&output.fragments, flow_root).unwrap();
    let child = fragment_for_dom(&output.fragments, child).unwrap();

    assert_eq!(
        flow_root.boxes.border_box.origin.y,
        host.boxes.content_box.origin.y + 10.0
    );
    assert_eq!(
        child.boxes.border_box.origin.y,
        flow_root.boxes.content_box.origin.y + 30.0
    );
}

#[test]
fn positive_min_height_prevents_empty_block_margin_pass_through() {
    let mut document = Document::new();
    let root = document.root();
    append_box(&mut document, root, "height:20px;margin-bottom:10px");
    append_box(
        &mut document,
        root,
        "min-height:1px;margin-top:30px;margin-bottom:40px",
    );
    let next = append_box(&mut document, root, "height:10px;margin-top:20px");

    let output = layout(&document, 300.0);
    let next = fragment_for_dom(&output.fragments, next).unwrap();

    assert_eq!(next.boxes.border_box.origin.y, 91.0);
}
