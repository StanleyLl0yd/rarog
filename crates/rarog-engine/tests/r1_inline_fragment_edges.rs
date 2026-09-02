use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{RenderOptions, render_html};
use rarog_layout::fragments_for_dom;
use rarog_paint::DisplayCommand;
use rarog_types::{Color, Size};

fn node_with<F>(document: &Document, predicate: F) -> NodeId
where
    F: Fn(&NodeKind) -> bool + Copy,
{
    fn find<F>(document: &Document, node: NodeId, predicate: F) -> Option<NodeId>
    where
        F: Fn(&NodeKind) -> bool + Copy,
    {
        if document
            .node(node)
            .is_some_and(|node| predicate(&node.kind))
        {
            return Some(node);
        }
        document
            .children(node)
            .unwrap_or(&[])
            .iter()
            .find_map(|child| find(document, *child, predicate))
    }

    find(document, document.root(), predicate).expect("fixture contains requested node")
}

#[test]
fn fragmented_inline_paints_horizontal_borders_only_at_outer_edges() {
    let output = render_html(
        "<div><span id=\"target\" style=\"display:inline;margin:0 3px 0 2px;border-width:0 5px 0 4px;border-color:#445566;padding:0 7px 0 6px;background:#112233\">ab cd ef</span></div>",
        RenderOptions {
            viewport: Size {
                width: 48.0,
                height: 180.0,
            },
            background: Color::WHITE,
        },
    )
    .unwrap();
    let inline = node_with(
        &output.document,
        |kind| matches!(kind, NodeKind::Element(element) if element.attributes.get("id").map(String::as_str) == Some("target")),
    );
    let fragment_count = fragments_for_dom(&output.layout.fragments, inline).len();
    let source = inline.index() as u64;
    let fills = output
        .display_list
        .command_ids()
        .iter()
        .copied()
        .zip(output.display_list.commands().iter().copied())
        .filter(|(id, command)| {
            id.source == source && matches!(command, DisplayCommand::FillRect { .. })
        })
        .count();

    assert!(fragment_count > 1);
    assert_eq!(fills, fragment_count + 2);
    assert!(output.display_list.validate().is_ok());
}
