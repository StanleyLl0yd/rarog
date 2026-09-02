use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{
    TextRange, fragment_for_dom, fragments_for_dom, is_grapheme_boundary, layout_document,
};
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
fn text_and_atomic_inline_box_share_one_line() {
    let mut document = Document::new();
    let host = host(&mut document);
    let text = append_text(&mut document, host, "ab ");
    let inline = append_box(&mut document, host, "display:inline;width:20px;height:10px");

    let output = layout(&document, 80.0);
    let text = fragments_for_dom(&output.fragments, text);
    let inline = fragment_for_dom(&output.fragments, inline).unwrap();

    assert_eq!(text.len(), 1);
    assert_eq!(text[0].boxes.content_box.origin.x, 0.0);
    assert_eq!(text[0].boxes.content_box.origin.y, 0.0);
    assert_eq!(inline.boxes.border_box.origin.x, 24.0);
    assert_eq!(inline.boxes.border_box.origin.y, 0.0);
}

#[test]
fn text_uses_remaining_width_then_continues_on_full_lines() {
    let mut document = Document::new();
    let host = host(&mut document);
    append_box(&mut document, host, "display:inline;width:20px;height:10px");
    let text = append_text(&mut document, host, "ab cd");

    let output = layout(&document, 44.0);
    let fragments = fragments_for_dom(&output.fragments, text);

    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].text_range, Some(TextRange::new(0, 3)));
    assert_eq!(fragments[0].boxes.content_box.origin.x, 20.0);
    assert_eq!(fragments[0].boxes.content_box.origin.y, 0.0);
    assert_eq!(fragments[1].text_range, Some(TextRange::new(3, 5)));
    assert_eq!(fragments[1].boxes.content_box.origin.x, 0.0);
    assert_eq!(fragments[1].boxes.content_box.origin.y, 18.0);
}

#[test]
fn mandatory_text_break_flushes_before_following_inline_box() {
    let mut document = Document::new();
    let host = host(&mut document);
    let text = append_text(&mut document, host, "a\nb");
    let inline = append_box(&mut document, host, "display:inline;width:20px;height:10px");

    let output = layout(&document, 80.0);
    let fragments = fragments_for_dom(&output.fragments, text);
    let inline = fragment_for_dom(&output.fragments, inline).unwrap();

    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].text_range, Some(TextRange::new(0, 2)));
    assert_eq!(fragments[1].text_range, Some(TextRange::new(2, 3)));
    assert_eq!(fragments[1].boxes.content_box.origin.y, 18.0);
    assert_eq!(inline.boxes.border_box.origin.x, 8.0);
    assert_eq!(inline.boxes.border_box.origin.y, 18.0);
}

#[test]
fn mixed_line_breaking_preserves_grapheme_safe_source_ranges() {
    let mut document = Document::new();
    let host = host(&mut document);
    append_box(&mut document, host, "display:inline;width:28px;height:10px");
    let text_value = "a\u{0301} bc";
    let text = append_text(&mut document, host, text_value);

    let output = layout(&document, 44.0);
    let fragments = fragments_for_dom(&output.fragments, text);

    assert!(fragments.len() >= 2);
    for fragment in fragments {
        let range = fragment.text_range.unwrap();
        assert!(is_grapheme_boundary(text_value, range.start));
        assert!(is_grapheme_boundary(text_value, range.end));
    }
}

#[test]
fn block_content_flushes_a_mixed_text_inline_line() {
    let mut document = Document::new();
    let host = host(&mut document);
    append_text(&mut document, host, "a");
    append_box(&mut document, host, "display:inline;width:20px;height:10px");
    let block = append_box(&mut document, host, "height:10px");

    let output = layout(&document, 80.0);
    let block = fragment_for_dom(&output.fragments, block).unwrap();

    assert_eq!(block.boxes.border_box.origin.y, 18.0);
}
