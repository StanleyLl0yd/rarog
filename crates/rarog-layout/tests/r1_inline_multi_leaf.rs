use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_layout::{fragments_for_dom, is_grapheme_boundary, layout_document};
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

#[test]
fn multi_leaf_stream_keeps_one_outer_fragment_per_line_and_source_order() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(
        &mut document,
        host,
        "display:inline;background:#112233;border-width:0 2px;padding:0 2px",
    );
    let first = append_text(&mut document, outer, "aa ");
    let inner = append_element(
        &mut document,
        outer,
        "display:inline;background:#334455;border-width:0 1px;padding:0 1px",
    );
    let inner_text_value = "b\u{0301}b cc ";
    let inner_text = append_text(&mut document, inner, inner_text_value);
    let last = append_text(&mut document, outer, "dd ee");

    let output = layout_document(
        &document,
        Size {
            width: 72.0,
            height: 240.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let inner_fragments = fragments_for_dom(&output.fragments, inner);
    let first_fragments = fragments_for_dom(&output.fragments, first);
    let inner_text_fragments = fragments_for_dom(&output.fragments, inner_text);
    let last_fragments = fragments_for_dom(&output.fragments, last);

    assert!(outer_fragments.len() >= 2);
    assert!(!inner_fragments.is_empty());
    assert!(!first_fragments.is_empty());
    assert!(!last_fragments.is_empty());
    assert!(
        outer_fragments
            .iter()
            .any(|fragment| fragment.children.len() >= 2)
    );

    for (ordinal, fragment) in outer_fragments.iter().enumerate() {
        assert_eq!(fragment.ordinal.index(), ordinal as u32);
        assert!(!fragment.children.is_empty());
    }
    for fragment in inner_text_fragments {
        let range = fragment.text_range.unwrap();
        assert!(is_grapheme_boundary(inner_text_value, range.start));
        assert!(is_grapheme_boundary(inner_text_value, range.end));
    }
}

#[test]
fn multi_leaf_stream_slices_outer_and_nested_edges_independently() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(
        &mut document,
        host,
        "display:inline;margin:0 3px 0 2px;border-width:0 5px 0 4px;padding:0 7px 0 6px",
    );
    append_text(&mut document, outer, "aa ");
    let inner = append_element(
        &mut document,
        outer,
        "display:inline;margin:0 2px 0 1px;border-width:0 4px 0 3px;padding:0 5px 0 2px",
    );
    append_text(&mut document, inner, "bb cc dd ee ");
    append_text(&mut document, outer, "ff gg");

    let output = layout_document(
        &document,
        Size {
            width: 64.0,
            height: 280.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let inner_fragments = fragments_for_dom(&output.fragments, inner);

    assert!(outer_fragments.len() >= 2);
    assert!(inner_fragments.len() >= 2);
    assert_eq!(outer_fragments[0].style.border_width.left, 4.0);
    assert_eq!(outer_fragments[0].style.border_width.right, 0.0);
    for fragment in outer_fragments
        .iter()
        .skip(1)
        .take(outer_fragments.len().saturating_sub(2))
    {
        assert_eq!(fragment.style.border_width.left, 0.0);
        assert_eq!(fragment.style.border_width.right, 0.0);
    }
    let outer_last = outer_fragments[outer_fragments.len() - 1];
    assert_eq!(outer_last.style.border_width.left, 0.0);
    assert_eq!(outer_last.style.border_width.right, 5.0);

    assert_eq!(inner_fragments[0].style.border_width.left, 3.0);
    assert_eq!(inner_fragments[0].style.border_width.right, 0.0);
    let inner_last = inner_fragments[inner_fragments.len() - 1];
    assert_eq!(inner_last.style.border_width.left, 0.0);
    assert_eq!(inner_last.style.border_width.right, 4.0);
}

#[test]
fn final_multi_leaf_fragment_shares_the_line_with_following_atomic_inline() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(&mut document, host, "display:inline");
    append_text(&mut document, outer, "aa bb cc ");
    let inner = append_element(&mut document, outer, "display:inline");
    append_text(&mut document, inner, "dd ");
    append_text(&mut document, outer, "e");
    let follower = append_element(
        &mut document,
        host,
        "display:inline;width:8px;height:16px;background:#778899",
    );

    let output = layout_document(
        &document,
        Size {
            width: 72.0,
            height: 220.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let follower_fragment = fragments_for_dom(&output.fragments, follower)[0];
    let last_outer = outer_fragments[outer_fragments.len() - 1];

    assert!(outer_fragments.len() >= 2);
    assert_eq!(
        follower_fragment.boxes.margin_box.origin.x,
        last_outer.boxes.margin_box.origin.x + last_outer.boxes.margin_box.size.width
    );
    assert!(
        follower_fragment.boxes.margin_box.origin.y
            < last_outer.boxes.margin_box.origin.y + last_outer.boxes.margin_box.size.height
    );
    assert!(
        last_outer.boxes.margin_box.origin.y
            < follower_fragment.boxes.margin_box.origin.y
                + follower_fragment.boxes.margin_box.size.height
    );
}
