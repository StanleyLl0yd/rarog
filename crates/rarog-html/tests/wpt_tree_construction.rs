use rarog_dom::{Document, Namespace, NodeId, NodeKind};
use rarog_html::parse;

const INBODY01: &str = include_str!("wpt/inbody01.dat");

fn cases(input: &str) -> Vec<(&str, &str)> {
    let mut output = Vec::new();
    let mut rest = input.trim_end();

    while !rest.is_empty() {
        assert!(rest.starts_with("#data\n"));
        let next = rest
            .find("\n\n#data\n")
            .map(|index| index + 2)
            .unwrap_or(rest.len());
        let block = &rest[..next].trim_end();
        let errors = block.find("\n#errors\n").expect("WPT case has #errors");
        let document = block.find("\n#document\n").expect("WPT case has #document");
        assert!(!block.contains("#document-fragment"));
        assert!(!block.contains("#script-on"));
        assert!(!block.contains("#script-off"));
        output.push((
            &block["#data\n".len()..errors],
            &block[document + "\n#document\n".len()..],
        ));
        rest = rest[next..].trim_start_matches('\n');
    }

    output
}

fn tree_dump(document: &Document) -> String {
    let mut output = String::new();
    for child in document.children(document.root()).unwrap_or(&[]) {
        dump_node(document, *child, 0, &mut output);
    }
    output.trim_end().to_owned()
}

fn dump_node(document: &Document, node: NodeId, depth: usize, output: &mut String) {
    let Some(node) = document.node(node) else {
        return;
    };
    let indent = "  ".repeat(depth);

    match &node.kind {
        NodeKind::Document => {
            for child in &node.children {
                dump_node(document, *child, depth, output);
            }
        }
        NodeKind::Element(element) => {
            let prefix = match &element.namespace {
                Namespace::Html => "",
                Namespace::Svg => "svg ",
                Namespace::MathMl => "math ",
                Namespace::Other(_) => "other ",
            };
            output.push_str(&format!(
                "| {indent}<{prefix}{}>\n",
                element.tag_name.as_str()
            ));
            for (name, value) in &element.attributes {
                output.push_str(&format!("| {indent}  {name}=\"{value}\"\n"));
            }
            for child in &node.children {
                dump_node(document, *child, depth + 1, output);
            }
        }
        NodeKind::Text(text) => {
            output.push_str(&format!("| {indent}\"{text}\"\n"));
        }
    }
}

#[test]
fn wpt_inbody01_tree_construction_matches_upstream() {
    let normalized = INBODY01.replace("\r\n", "\n");
    let cases = cases(&normalized);
    assert_eq!(cases.len(), 4);

    for (index, (source, expected)) in cases.into_iter().enumerate() {
        let actual = tree_dump(&parse(source));
        assert_eq!(actual, expected, "WPT inbody01 case {}", index + 1);
    }
}
