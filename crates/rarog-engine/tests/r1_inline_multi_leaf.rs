use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_layout::fragments_for_dom;
use rarog_types::{Color, Size};

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 72.0,
            height: 240.0,
        },
        background: Color::WHITE,
    }
}

fn node_with_id(document: &Document, id: &str) -> NodeId {
    fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
        if document.node(node).is_some_and(|node| {
            matches!(&node.kind, NodeKind::Element(element) if element.attributes.get("id").map(String::as_str) == Some(id))
        }) {
            return Some(node);
        }
        document
            .children(node)
            .unwrap_or(&[])
            .iter()
            .find_map(|child| find(document, *child, id))
    }

    find(document, document.root(), id).expect("fixture contains requested id")
}

#[test]
fn multi_leaf_inline_stream_keeps_unique_display_identity() {
    let output = render_html(
        "<div><span id=\"outer\" style=\"display:inline;background:#112233\">aa <em id=\"inner\" style=\"display:inline;background:#334455\">bb cc dd </em>ee ff</span></div>",
        options(),
    )
    .unwrap();
    let outer = node_with_id(&output.document, "outer");
    let inner = node_with_id(&output.document, "inner");

    assert!(fragments_for_dom(&output.layout.fragments, outer).len() >= 2);
    assert!(!fragments_for_dom(&output.layout.fragments, inner).is_empty());
    assert!(output.display_list.has_unique_ids());
    assert!(output.display_list.validate().is_ok());
}

#[test]
fn multi_leaf_nested_style_change_matches_fresh_render() {
    let source = "<div><span id=\"outer\" style=\"display:inline;background:#112233\">aa <em id=\"inner\" style=\"display:inline;background:#334455\">bb cc dd </em>ee ff</span></div>";
    let expected = "<div><span id=\"outer\" style=\"display:inline;background:#112233\">aa <em id=\"inner\" style=\"display:inline;background:#778899\">bb cc dd </em>ee ff</span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let inner = node_with_id(session.document(), "inner");

    session
        .document_mut()
        .set_attribute(inner, "style", "display:inline;background:#778899")
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert!(report.retained_display_list);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
    assert_eq!(
        session.layout().fragments.snapshot(),
        fresh.layout.fragments.snapshot()
    );
}
