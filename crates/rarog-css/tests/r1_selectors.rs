use rarog_css::{
    InvalidationSet, Specificity, StyleSet, StyleSource, Stylesheet, computed_style, parse_selector,
};
use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use std::collections::BTreeMap;

fn append_element(
    document: &mut Document,
    parent: NodeId,
    tag: &str,
    attributes: &[(&str, &str)],
) -> NodeId {
    let attributes = attributes
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect::<BTreeMap<_, _>>();
    document
        .append_new(
            parent,
            NodeKind::Element(ElementData::html(tag).with_attributes(attributes)),
        )
        .unwrap()
}

#[test]
fn structural_selector_slice_matches_and_counts_specificity() {
    let mut document = Document::new();
    let root = document.root();
    let section = append_element(&mut document, root, "section", &[("class", "theme")]);
    let first = append_element(&mut document, section, "div", &[("data-state", "ready")]);
    let span = append_element(&mut document, first, "span", &[("class", "item")]);
    let second = append_element(&mut document, section, "div", &[("data-state", "ready")]);

    let selector =
        parse_selector("section.theme > div[data-state=ready]:first-child span.item").unwrap();
    assert_eq!(
        selector.specificity(),
        Specificity {
            ids: 0,
            classes: 4,
            types: 3,
        }
    );
    assert!(selector.matches(&document, span));
    assert!(!selector.matches(&document, second));
}

#[test]
fn sibling_combinators_ignore_text_nodes() {
    let mut document = Document::new();
    let root = document.root();
    let lead = append_element(&mut document, root, "div", &[("class", "lead")]);
    document
        .append_new(root, NodeKind::Text("text".into()))
        .unwrap();
    let adjacent = append_element(&mut document, root, "div", &[("class", "target")]);
    let middle = append_element(&mut document, root, "div", &[("class", "middle")]);
    let later = append_element(&mut document, root, "div", &[("class", "target")]);

    let next = parse_selector(".lead + .target").unwrap();
    let subsequent = parse_selector(".lead ~ .target").unwrap();

    assert!(next.matches(&document, adjacent));
    assert!(!next.matches(&document, later));
    assert!(subsequent.matches(&document, adjacent));
    assert!(subsequent.matches(&document, later));
    assert!(!subsequent.matches(&document, lead));
    assert!(!subsequent.matches(&document, middle));
}

#[test]
fn attribute_and_structural_pseudo_rules_reach_computed_style() {
    let mut document = Document::new();
    let root = document.root();
    let list = append_element(&mut document, root, "ul", &[("class", "list")]);
    let first = append_element(&mut document, list, "li", &[("data-state", "idle")]);
    let last = append_element(&mut document, list, "li", &[("data-state", "ready")]);
    let stylesheet = Stylesheet::parse(
        StyleSource::author(1, "test"),
        ".list > li[data-state=ready]:last-child { width:42px; }",
    );
    let styles = StyleSet {
        stylesheets: vec![stylesheet],
    };

    assert_eq!(computed_style(&document, first, &styles).width, None);
    assert_eq!(computed_style(&document, last, &styles).width, Some(42.0));
}

#[test]
fn selector_dependencies_cover_attributes_ancestors_and_sibling_position() {
    let mut document = Document::new();
    let root = document.root();
    let parent = append_element(&mut document, root, "section", &[("class", "theme")]);
    let first = append_element(&mut document, parent, "div", &[("data-state", "ready")]);
    let stylesheet = Stylesheet::parse(
        StyleSource::author(1, "test"),
        ".theme > div[data-state=ready]:last-child { width:42px; }",
    );
    let styles = StyleSet {
        stylesheets: vec![stylesheet],
    };

    let generation = document.generation();
    document.set_attribute(first, "data-state", "idle").unwrap();
    let invalidation =
        InvalidationSet::from_document_since_with_styles(&document, generation, &styles);
    assert!(invalidation.entries.contains_key(&first));

    let generation = document.generation();
    document.remove_attribute(parent, "class").unwrap();
    let invalidation =
        InvalidationSet::from_document_since_with_styles(&document, generation, &styles);
    assert!(invalidation.entries.contains_key(&first));

    document.set_attribute(parent, "class", "theme").unwrap();
    let generation = document.generation();
    let second = append_element(&mut document, parent, "div", &[("data-state", "ready")]);
    let invalidation =
        InvalidationSet::from_document_since_with_styles(&document, generation, &styles);
    assert!(invalidation.entries.contains_key(&first));
    assert!(invalidation.entries.contains_key(&second));
}

#[test]
fn root_pseudo_matches_only_document_element() {
    let mut document = Document::new();
    let root = document.root();
    let html = append_element(&mut document, root, "html", &[]);
    let body = append_element(&mut document, html, "body", &[]);
    let selector = parse_selector(":root").unwrap();

    assert!(selector.matches(&document, html));
    assert!(!selector.matches(&document, body));
}
