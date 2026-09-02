use rarog_css::{CssWideKeyword, StyleSet, StyleSource, Stylesheet, computed_style};
use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_types::Color;
use std::collections::BTreeMap;

fn append_element(
    document: &mut Document,
    parent: NodeId,
    tag: &str,
    attributes: &[(&str, &str)],
) -> NodeId {
    let mut attrs = BTreeMap::new();
    for (name, value) in attributes {
        attrs.insert((*name).into(), (*value).into());
    }
    document
        .append_new(
            parent,
            NodeKind::Element(ElementData::html(tag).with_attributes(attrs)),
        )
        .unwrap()
}

fn author_styles(css: &str) -> StyleSet {
    StyleSet {
        stylesheets: vec![Stylesheet::parse(StyleSource::author(1, "test"), css)],
    }
}

#[test]
fn important_beats_specificity_and_inline_normal() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(
        &mut document,
        root,
        "div",
        &[
            ("id", "hero"),
            ("class", "card"),
            ("style", "color:#445566"),
        ],
    );
    let styles = author_styles(".card { color:#112233 !important; } #hero { color:#334455; }");

    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0x11, 0x22, 0x33)
    );
}

#[test]
fn inline_important_beats_author_important_but_not_user_agent_important() {
    let mut document = Document::new();
    let root = document.root();
    let node = append_element(
        &mut document,
        root,
        "div",
        &[("id", "hero"), ("style", "color:#445566 !important")],
    );
    let author = Stylesheet::parse(
        StyleSource::author(1, "author"),
        "#hero { color:#112233 !important; }",
    );
    let styles = StyleSet {
        stylesheets: vec![author],
    };
    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0x44, 0x55, 0x66)
    );

    let user_agent = Stylesheet::parse(
        StyleSource::user_agent(),
        "#hero { color:#778899 !important; }",
    );
    let styles = StyleSet {
        stylesheets: vec![user_agent, styles.stylesheets[0].clone()],
    };
    assert_eq!(
        computed_style(&document, node, &styles).color,
        Color::rgb(0x77, 0x88, 0x99)
    );
}

#[test]
fn inherited_color_and_css_wide_values_resolve_at_computed_value_time() {
    let mut document = Document::new();
    let root = document.root();
    let parent = append_element(&mut document, root, "section", &[("id", "parent")]);
    let inherited = append_element(&mut document, parent, "div", &[("id", "inherited")]);
    let explicit_inherit = append_element(&mut document, parent, "div", &[("id", "width-inherit")]);
    let initial = append_element(&mut document, parent, "div", &[("id", "initial")]);
    let unset = append_element(&mut document, parent, "div", &[("id", "unset")]);
    let styles = author_styles(
        "#parent { color:#112233; width:42px; margin:7px; } \
         #width-inherit { width:inherit; } \
         #initial { color:initial; width:initial; margin:initial; } \
         #unset { color:unset; width:unset; }",
    );

    assert_eq!(
        computed_style(&document, inherited, &styles).color,
        Color::rgb(0x11, 0x22, 0x33)
    );
    assert_eq!(
        computed_style(&document, explicit_inherit, &styles).width,
        Some(42.0)
    );

    let initial_style = computed_style(&document, initial, &styles);
    assert_eq!(initial_style.color, Color::BLACK);
    assert_eq!(initial_style.width, None);
    assert_eq!(initial_style.margin.top, 0.0);
    assert_eq!(initial_style.margin.right, 0.0);
    assert_eq!(initial_style.margin.bottom, 0.0);
    assert_eq!(initial_style.margin.left, 0.0);

    let unset_style = computed_style(&document, unset, &styles);
    assert_eq!(unset_style.color, Color::rgb(0x11, 0x22, 0x33));
    assert_eq!(unset_style.width, None);

    let _keywords = [
        CssWideKeyword::Initial,
        CssWideKeyword::Inherit,
        CssWideKeyword::Unset,
    ];
}
