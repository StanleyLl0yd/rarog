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
fn nested_inline_chain_fragments_each_owner_on_the_same_lines() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(
        &mut document,
        host,
        "display:inline;background:#112233;border-width:0 3px 0 2px;padding:0 2px",
    );
    let inner = append_element(
        &mut document,
        outer,
        "display:inline;background:#334455;border-width:0 2px 0 1px;padding:0 1px",
    );
    let text_value = "a\u{0301} bc de fg";
    let text = append_text(&mut document, inner, text_value);

    let output = layout_document(
        &document,
        Size {
            width: 48.0,
            height: 200.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let inner_fragments = fragments_for_dom(&output.fragments, inner);
    let text_fragments = fragments_for_dom(&output.fragments, text);

    assert!(outer_fragments.len() >= 2);
    assert_eq!(outer_fragments.len(), inner_fragments.len());
    assert_eq!(inner_fragments.len(), text_fragments.len());

    for (ordinal, ((outer_fragment, inner_fragment), text_fragment)) in outer_fragments
        .iter()
        .zip(&inner_fragments)
        .zip(&text_fragments)
        .enumerate()
    {
        assert_eq!(outer_fragment.ordinal.index(), ordinal as u32);
        assert_eq!(inner_fragment.ordinal.index(), ordinal as u32);
        assert_eq!(text_fragment.ordinal.index(), ordinal as u32);
        assert_eq!(outer_fragment.children.len(), 1);
        assert_eq!(outer_fragment.children[0].dom_node, Some(inner));
        assert_eq!(inner_fragment.children.len(), 1);
        assert_eq!(inner_fragment.children[0].dom_node, Some(text));
        let range = text_fragment.text_range.unwrap();
        assert!(is_grapheme_boundary(text_value, range.start));
        assert!(is_grapheme_boundary(text_value, range.end));
    }
}

#[test]
fn nested_inline_chain_slices_each_owners_horizontal_edges() {
    let mut document = Document::new();
    let root = document.root();
    let host = append_element(&mut document, root, "");
    let outer = append_element(
        &mut document,
        host,
        "display:inline;margin:0 3px 0 2px;border-width:0 5px 0 4px;padding:0 7px 0 6px",
    );
    let inner = append_element(
        &mut document,
        outer,
        "display:inline;margin:0 2px 0 1px;border-width:0 4px 0 3px;padding:0 5px 0 2px",
    );
    append_text(&mut document, inner, "ab cd ef gh");

    let output = layout_document(
        &document,
        Size {
            width: 56.0,
            height: 200.0,
        },
    );
    let outer_fragments = fragments_for_dom(&output.fragments, outer);
    let inner_fragments = fragments_for_dom(&output.fragments, inner);

    assert!(outer_fragments.len() >= 2);
    assert_eq!(outer_fragments.len(), inner_fragments.len());

    let outer_first = outer_fragments[0];
    let inner_first = inner_fragments[0];
    assert_eq!(outer_first.style.border_width.left, 4.0);
    assert_eq!(outer_first.style.border_width.right, 0.0);
    assert_eq!(inner_first.style.border_width.left, 3.0);
    assert_eq!(inner_first.style.border_width.right, 0.0);

    for fragment in outer_fragments
        .iter()
        .skip(1)
        .take(outer_fragments.len().saturating_sub(2))
    {
        assert_eq!(fragment.style.border_width.left, 0.0);
        assert_eq!(fragment.style.border_width.right, 0.0);
    }
    for fragment in inner_fragments
        .iter()
        .skip(1)
        .take(inner_fragments.len().saturating_sub(2))
    {
        assert_eq!(fragment.style.border_width.left, 0.0);
        assert_eq!(fragment.style.border_width.right, 0.0);
    }

    let outer_last = outer_fragments[outer_fragments.len() - 1];
    let inner_last = inner_fragments[inner_fragments.len() - 1];
    assert_eq!(outer_last.style.border_width.left, 0.0);
    assert_eq!(outer_last.style.border_width.right, 5.0);
    assert_eq!(inner_last.style.border_width.left, 0.0);
    assert_eq!(inner_last.style.border_width.right, 4.0);
}
