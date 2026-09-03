use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_layout::fragments_for_dom;
use rarog_types::{Color, Size};

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 32.0,
            height: 120.0,
        },
        background: Color::WHITE,
    }
}

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
fn fragmented_inline_background_keeps_unique_display_identity() {
    let output = render_html(
        "<div><span style=\"display:inline;background:#112233\">ab cd ef</span></div>",
        options(),
    )
    .unwrap();
    let inline = node_with(
        &output.document,
        |kind| matches!(kind, NodeKind::Element(element) if element.tag_name.as_str() == "span"),
    );

    assert!(fragments_for_dom(&output.layout.fragments, inline).len() > 1);
    assert!(output.display_list.has_unique_ids());
    assert!(output.display_list.validate().is_ok());
}

#[test]
fn text_growth_reflows_inline_fragments_to_match_fresh_render() {
    let source = "<div><span style=\"display:inline;background:#112233\">ab</span></div>";
    let expected = "<div><span style=\"display:inline;background:#112233\">ab cd ef</span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let text = node_with(session.document(), |kind| matches!(kind, NodeKind::Text(_)));

    session.document_mut().set_text(text, "ab cd ef").unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}

#[test]
fn fragmented_inline_style_change_uses_retained_flow_relayout() {
    let source =
        "<div><span id=\"chip\" style=\"display:inline;background:#112233\">ab cd ef</span></div>";
    let expected =
        "<div><span id=\"chip\" style=\"display:inline;background:#778899\">ab cd ef</span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let inline = node_with(
        session.document(),
        |kind| matches!(kind, NodeKind::Element(element) if element.attributes.get("id").map(String::as_str) == Some("chip")),
    );

    session
        .document_mut()
        .set_attribute(inline, "style", "display:inline;background:#778899")
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert!(report.retained_display_list);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
