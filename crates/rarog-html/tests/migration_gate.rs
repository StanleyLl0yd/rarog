use rarog_dom::{Document, NodeId, NodeKind};
use rarog_html::{parse, parse_standards};

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
fn canonical_and_standards_entry_points_are_identical() {
    let source = "<!doctype html><html><body><p id=\"x\">hello</p></body></html>";

    assert_eq!(parse(source).snapshot(), parse_standards(source).snapshot());
}

#[test]
fn canonical_parser_resolves_character_references() {
    let document = parse("<!doctype html><html><body><p>A&amp;B&nbsp;C</p></body></html>");
    let paragraph = find_element(&document, "p").unwrap();

    assert_eq!(descendant_text(&document, paragraph), "A&B\u{00a0}C");
}

#[test]
fn canonical_parser_applies_table_insertion_rules() {
    let document =
        parse("<!doctype html><html><body><table><tr><td>x</td></tr></table></body></html>");

    assert!(find_element(&document, "tbody").is_some());
}
