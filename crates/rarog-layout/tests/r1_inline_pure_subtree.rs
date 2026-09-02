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

fn append_text(document: &mut Document, parent: NodeId, value: &str) -> NodeId {
    document
        .append_new(parent, NodeKind::Text(value.into()))
        .unwrap()
}

#[test]
fn nested_multi_leaf_owner_reuses_one_fragment_per_line() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(&mut document, host, "display:inline;background:#111111");
    append_text(&mut document, outer, "aa ");
    let inner = append_element(
        &mut document,
        outer,
        "display:inline;background:#222222;border-width:0 2px;padding:0 2px",
    );
    let first = append_text(&mut document, inner, "bb ");
    let strong = append_element(&mut document, inner, "display:inline;background:#333333");
    let strong_text = append_text(&mut document, strong, "c\u{0301}c dd ");
    let last = append_text(&mut document, inner, "ee ff ");
    append_text(&mut document, outer, "gg");

    let output = layout_document(
        &document,
        Size {
            width: 72.0,
            height: 320.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let inner_fragments = fragments_for_dom(&output.fragments, inner);
    let first_fragments = fragments_for_dom(&output.fragments, first);
    let strong_fragments = fragments_for_dom(&output.fragments, strong);
    let strong_text_fragments = fragments_for_dom(&output.fragments, strong_text);
    let last_fragments = fragments_for_dom(&output.fragments, last);

    assert!(outer_fragments.len() >= 2);
    assert!(inner_fragments.len() >= 2);
    assert!(!first_fragments.is_empty());
    assert!(!strong_fragments.is_empty());
    assert!(!last_fragments.is_empty());
    assert!(
        inner_fragments
            .iter()
            .any(|fragment| fragment.children.len() >= 2)
    );
    for (ordinal, fragment) in inner_fragments.iter().enumerate() {
        assert_eq!(fragment.ordinal.index(), ordinal as u32);
    }
    let value = "c\u{0301}c dd ";
    for fragment in strong_text_fragments {
        let range = fragment.text_range.unwrap();
        assert!(is_grapheme_boundary(value, range.start));
        assert!(is_grapheme_boundary(value, range.end));
    }
}

#[test]
fn nested_multi_leaf_owner_slices_edges_across_all_descendants() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(&mut document, host, "display:inline");
    append_text(&mut document, outer, "aa ");
    let inner = append_element(
        &mut document,
        outer,
        "display:inline;margin:0 3px 0 2px;border-width:0 5px 0 4px;padding:0 7px 0 6px",
    );
    append_text(&mut document, inner, "bb ");
    let nested = append_element(&mut document, inner, "display:inline");
    append_text(&mut document, nested, "cc dd ee ");
    append_text(&mut document, inner, "ff gg hh ");
    append_text(&mut document, outer, "ii");

    let output = layout_document(
        &document,
        Size {
            width: 64.0,
            height: 360.0,
        },
    );
    let fragments = fragments_for_dom(&output.fragments, inner);

    assert!(fragments.len() >= 3);
    assert_eq!(fragments[0].style.border_width.left, 4.0);
    assert_eq!(fragments[0].style.border_width.right, 0.0);
    for fragment in fragments
        .iter()
        .skip(1)
        .take(fragments.len().saturating_sub(2))
    {
        assert_eq!(fragment.style.border_width.left, 0.0);
        assert_eq!(fragment.style.border_width.right, 0.0);
    }
    let last = fragments[fragments.len() - 1];
    assert_eq!(last.style.border_width.left, 0.0);
    assert_eq!(last.style.border_width.right, 5.0);
}

#[test]
fn sibling_nested_owners_preserve_source_order_without_duplicate_owner_fragments() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(&mut document, host, "display:inline");
    let left = append_element(&mut document, outer, "display:inline;background:#111111");
    append_text(&mut document, left, "aa bb ");
    let middle = append_text(&mut document, outer, "cc ");
    let right = append_element(&mut document, outer, "display:inline;background:#222222");
    append_text(&mut document, right, "dd ee");

    let output = layout_document(
        &document,
        Size {
            width: 120.0,
            height: 160.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let left_fragments = fragments_for_dom(&output.fragments, left);
    let middle_fragments = fragments_for_dom(&output.fragments, middle);
    let right_fragments = fragments_for_dom(&output.fragments, right);

    assert_eq!(outer_fragments.len(), 1);
    assert_eq!(left_fragments.len(), 1);
    assert_eq!(middle_fragments.len(), 1);
    assert_eq!(right_fragments.len(), 1);
    let children = &outer_fragments[0].children;
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].dom_node, Some(left));
    assert_eq!(children[1].dom_node, Some(middle));
    assert_eq!(children[2].dom_node, Some(right));
}
