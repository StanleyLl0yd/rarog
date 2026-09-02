use rarog_dom::{Document, NodeId, NodeKind};
use rarog_html::{parse_standards_with_diagnostics, parse_with_diagnostics};

fn find_element(document: &Document, tag_name: &str) -> Option<NodeId> {
    let mut stack = vec![document.root()];
    while let Some(node) = stack.pop() {
        let Some(current) = document.node(node) else {
            continue;
        };
        if let NodeKind::Element(element) = &current.kind {
            if element.tag_name.as_str() == tag_name {
                return Some(node);
            }
        }
        stack.extend(current.children.iter().rev().copied());
    }
    None
}

fn descendant_text(document: &Document, root: NodeId) -> String {
    let mut output = String::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let Some(current) = document.node(node) else {
            continue;
        };
        match &current.kind {
            NodeKind::Text(text) => output.push_str(text),
            NodeKind::Document | NodeKind::Element(_) => {
                stack.extend(current.children.iter().rev().copied());
            }
        }
    }
    output
}

#[test]
fn well_formed_bootstrap_subset_matches_standards_tree() {
    let fixtures = [
        "<!doctype html><html><head><title>x</title></head><body><p id=\"x\">hello</p></body></html>",
        "<!doctype html><html><head></head><body><div><span>x</span><br><img src=\"x\"></div></body></html>",
    ];

    for source in fixtures {
        let bootstrap = parse_with_diagnostics(source);
        let standards = parse_standards_with_diagnostics(source);
        assert_eq!(bootstrap.document.snapshot(), standards.document.snapshot());
    }
}

#[test]
fn character_references_are_an_intentional_standards_change() {
    let source = "<!doctype html><html><head></head><body><p>A&amp;B&nbsp;C</p></body></html>";
    let bootstrap = parse_with_diagnostics(source);
    let standards = parse_standards_with_diagnostics(source);
    let bootstrap_p = find_element(&bootstrap.document, "p").unwrap();
    let standards_p = find_element(&standards.document, "p").unwrap();

    assert_eq!(
        descendant_text(&bootstrap.document, bootstrap_p),
        "A&amp;B&nbsp;C"
    );
    assert_eq!(
        descendant_text(&standards.document, standards_p),
        "A&B\u{00a0}C"
    );
}

#[test]
fn table_insertion_is_an_intentional_standards_change() {
    let source =
        "<!doctype html><html><head></head><body><table><tr><td>x</td></tr></table></body></html>";
    let bootstrap = parse_with_diagnostics(source);
    let standards = parse_standards_with_diagnostics(source);

    assert!(find_element(&bootstrap.document, "tbody").is_none());
    assert!(find_element(&standards.document, "tbody").is_some());
}
