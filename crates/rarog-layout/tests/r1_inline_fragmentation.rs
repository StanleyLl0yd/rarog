use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{
    FragmentKind, fragment_for_dom, fragments_for_dom, is_grapheme_boundary, layout_document,
};
use rarog_types::Size;
use std::collections::BTreeMap;

fn append_element(document: &mut Document, parent: NodeId, style: &str) -> NodeId {
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
    append_element(document, root, "")
}

fn layout(document: &Document, width: f32) -> rarog_layout::LayoutOutput {
    layout_document(
        document,
        Size {
            width,
            height: 240.0,
        },
    )
}

#[test]
fn unsized_inline_text_container_fragments_across_lines() {
    let mut document = Document::new();
    let host = host(&mut document);
    let inline = append_element(&mut document, host, "display:inline;background:#112233");
    let text = append_text(&mut document, inline, "ab cd ef");

    let output = layout(&document, 32.0);
    let inline_fragments = fragments_for_dom(&output.fragments, inline);
    let text_fragments = fragments_for_dom(&output.fragments, text);

    assert_eq!(inline_fragments.len(), 3);
    assert_eq!(text_fragments.len(), 3);
    for (index, fragment) in inline_fragments.iter().enumerate() {
        assert_eq!(fragment.kind, FragmentKind::Box);
        assert_eq!(fragment.ordinal.index(), index as u32);
        assert_eq!(fragment.children.len(), 1);
        assert_eq!(fragment.children[0].dom_node, Some(text));
        assert_eq!(fragment.children[0].ordinal.index(), index as u32);
    }
    assert_eq!(inline_fragments[0].boxes.border_box.origin.y, 0.0);
    assert_eq!(inline_fragments[1].boxes.border_box.origin.y, 18.0);
    assert_eq!(inline_fragments[2].boxes.border_box.origin.y, 36.0);
}

#[test]
fn inline_container_fragment_ranges_stay_grapheme_safe() {
    let mut document = Document::new();
    let host = host(&mut document);
    let inline = append_element(&mut document, host, "display:inline");
    let value = "a\u{0301} bc de";
    let text = append_text(&mut document, inline, value);

    let output = layout(&document, 24.0);
    let fragments = fragments_for_dom(&output.fragments, text);

    assert!(fragments.len() >= 2);
    for fragment in fragments {
        let range = fragment.text_range.unwrap();
        assert!(is_grapheme_boundary(value, range.start));
        assert!(is_grapheme_boundary(value, range.end));
    }
}

#[test]
fn final_inline_fragment_shares_line_with_following_atomic_inline_box() {
    let mut document = Document::new();
    let host = host(&mut document);
    let inline = append_element(&mut document, host, "display:inline");
    append_text(&mut document, inline, "ab cd");
    let atomic = append_element(&mut document, host, "display:inline;width:12px;height:10px");

    let output = layout(&document, 32.0);
    let inline_fragments = fragments_for_dom(&output.fragments, inline);
    let atomic = fragment_for_dom(&output.fragments, atomic).unwrap();

    assert_eq!(inline_fragments.len(), 2);
    assert_eq!(inline_fragments[1].boxes.border_box.origin.y, 18.0);
    assert_eq!(atomic.boxes.border_box.origin.y, 22.0);
    assert_eq!(atomic.boxes.border_box.origin.x, 16.0);
}

#[test]
fn explicit_inline_sizing_keeps_the_atomic_fallback() {
    let mut document = Document::new();
    let host = host(&mut document);
    let inline = append_element(
        &mut document,
        host,
        "display:inline;width:24px;height:20px;background:#112233",
    );
    append_text(&mut document, inline, "ab cd ef");

    let output = layout(&document, 32.0);
    let fragments = fragments_for_dom(&output.fragments, inline);

    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].boxes.content_box.size.width, 24.0);
    assert_eq!(fragments[0].boxes.content_box.size.height, 20.0);
}

#[test]
fn following_block_starts_after_all_inline_container_fragments() {
    let mut document = Document::new();
    let host = host(&mut document);
    let inline = append_element(&mut document, host, "display:inline");
    append_text(&mut document, inline, "ab cd ef");
    let block = append_element(&mut document, host, "height:10px");

    let output = layout(&document, 32.0);
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(block.boxes.border_box.origin.y, 54.0);
}
