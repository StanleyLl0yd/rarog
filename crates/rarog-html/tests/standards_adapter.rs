use rarog_dom::{Document, Namespace, NodeId, NodeKind};
use rarog_html::parse_standards_with_diagnostics;

fn find_element(document: &Document, tag_name: &str) -> Option<NodeId> {
    let mut stack = vec![document.root()];
    while let Some(node) = stack.pop() {
        if let Some(current) = document.node(node) {
            if let NodeKind::Element(element) = &current.kind {
                if element.tag_name.as_str() == tag_name {
                    return Some(node);
                }
            }
            stack.extend(current.children.iter().rev().copied());
        }
    }
    None
}

fn descendant_text(document: &Document, root: NodeId) -> String {
    let mut output = String::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let Some(current) = document.node(node) {
            match &current.kind {
                NodeKind::Text(text) => output.push_str(text),
                NodeKind::Document | NodeKind::Element(_) => {
                    stack.extend(current.children.iter().rev().copied());
                }
            }
        }
    }
    output
}

#[test]
fn standards_adapter_inserts_document_structure_and_decodes_entities() {
    let output = parse_standards_with_diagnostics("<title>T &amp; C</title><p id=x>A&nbsp;B</p>");

    assert_eq!(output.document.validate_invariants(), Ok(()));
    assert!(find_element(&output.document, "html").is_some());
    assert!(find_element(&output.document, "head").is_some());
    assert!(find_element(&output.document, "body").is_some());

    let title = find_element(&output.document, "title").unwrap();
    let paragraph = find_element(&output.document, "p").unwrap();
    assert_eq!(descendant_text(&output.document, title), "T & C");
    assert_eq!(descendant_text(&output.document, paragraph), "A\u{00a0}B");
    let NodeKind::Element(element) = &output.document.node(paragraph).unwrap().kind else {
        panic!("expected paragraph element");
    };
    assert_eq!(element.attributes.get("id").map(String::as_str), Some("x"));
}

#[test]
fn standards_adapter_runs_tree_builder_rules() {
    let output = parse_standards_with_diagnostics("<table><tr><td>x</td></tr></table>");

    assert_eq!(output.document.validate_invariants(), Ok(()));
    assert!(find_element(&output.document, "tbody").is_some());
    assert!(find_element(&output.document, "td").is_some());
}

#[test]
fn standards_adapter_preserves_foreign_content_namespace() {
    let output = parse_standards_with_diagnostics("<svg><circle></circle></svg>");
    let svg = find_element(&output.document, "svg").unwrap();
    let circle = find_element(&output.document, "circle").unwrap();

    let NodeKind::Element(svg) = &output.document.node(svg).unwrap().kind else {
        panic!("expected svg element");
    };
    let NodeKind::Element(circle) = &output.document.node(circle).unwrap().kind else {
        panic!("expected circle element");
    };
    assert_eq!(svg.namespace, Namespace::Svg);
    assert_eq!(circle.namespace, Namespace::Svg);
}
