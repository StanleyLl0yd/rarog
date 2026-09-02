use rarog_css::{CascadeLayer, StyleSet, StyleSource, Stylesheet, computed_style};
use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_types::Color;
use std::collections::BTreeMap;

fn append_element(document: &mut Document, parent: NodeId, attributes: &[(&str, &str)]) -> NodeId {
    let mut attrs = BTreeMap::new();
    for (name, value) in attributes {
        attrs.insert((*name).into(), (*value).into());
    }
    document
        .append_new(
            parent,
            NodeKind::Element(ElementData::html("div").with_attributes(attrs)),
        )
        .unwrap()
}

fn author_source(id: u32, layer: u16, label: &str) -> StyleSource {
    let mut source = StyleSource::author(id, label);
    source.layer = CascadeLayer(layer);
    source
}

#[test]
fn revert_removes_the_author_origin_instead_of_falling_to_an_earlier_author_rule() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(&mut document, root, &[("id", "hero")]);
    let user_agent = Stylesheet::parse(StyleSource::user_agent(), "#hero { color:#112233; }");
    let author = Stylesheet::parse(
        StyleSource::author(1, "author"),
        "#hero { color:#334455; } #hero { color:revert; }",
    );
    let styles = StyleSet {
        stylesheets: vec![user_agent, author],
    };

    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0x11, 0x22, 0x33)
    );
}

#[test]
fn user_agent_revert_behaves_like_unset() {
    let mut document = Document::new();
    let root = document.root();
    let parent = append_element(&mut document, root, &[("id", "parent")]);
    let child = append_element(&mut document, parent, &[("id", "child")]);
    let author = Stylesheet::parse(
        StyleSource::author(1, "author"),
        "#parent { color:#445566; }",
    );
    let user_agent = Stylesheet::parse(StyleSource::user_agent(), "#child { color:revert; }");
    let styles = StyleSet {
        stylesheets: vec![user_agent, author],
    };

    assert_eq!(
        computed_style(&document, child, &styles).color,
        Color::rgb(0x44, 0x55, 0x66)
    );
}

#[test]
fn revert_layer_falls_back_to_the_previous_author_layer() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(&mut document, root, &[("id", "hero")]);
    let base = Stylesheet::parse(author_source(1, 0, "base"), "#hero { color:#112233; }");
    let override_layer = Stylesheet::parse(
        author_source(2, 1, "override"),
        "#hero { color:#445566; } #hero { color:revert-layer; }",
    );
    let styles = StyleSet {
        stylesheets: vec![base, override_layer],
    };

    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0x11, 0x22, 0x33)
    );
}

#[test]
fn important_revert_layer_respects_reversed_layer_order() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(&mut document, root, &[("id", "hero")]);
    let first = Stylesheet::parse(
        author_source(1, 0, "first"),
        "#hero { color:revert-layer !important; }",
    );
    let second = Stylesheet::parse(
        author_source(2, 1, "second"),
        "#hero { color:#778899 !important; }",
    );
    let styles = StyleSet {
        stylesheets: vec![first, second],
    };

    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0x77, 0x88, 0x99)
    );
}

#[test]
fn inline_revert_layer_falls_back_to_author_stylesheet_scope() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(
        &mut document,
        root,
        &[("id", "hero"), ("style", "color:revert-layer")],
    );
    let styles = StyleSet {
        stylesheets: vec![Stylesheet::parse(
            StyleSource::author(1, "author"),
            "#hero { color:#abcdef; }",
        )],
    };

    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0xab, 0xcd, 0xef)
    );
}

#[test]
fn revert_keywords_expand_across_supported_shorthands() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(&mut document, root, &[("id", "hero")]);
    let user_agent = Stylesheet::parse(StyleSource::user_agent(), "#hero { margin:3px; }");
    let author = Stylesheet::parse(
        StyleSource::author(1, "author"),
        "#hero { margin:9px; margin:revert; }",
    );
    let styles = StyleSet {
        stylesheets: vec![user_agent, author],
    };
    let style = computed_style(&document, node, &styles);

    assert_eq!(style.margin.top, 3.0);
    assert_eq!(style.margin.right, 3.0);
    assert_eq!(style.margin.bottom, 3.0);
    assert_eq!(style.margin.left, 3.0);
}
