use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_layout::fragments_for_dom;
use rarog_types::{Color, Size};

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 72.0,
            height: 280.0,
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
fn pure_inline_subtree_keeps_unique_fragment_and_display_identity() {
    let source = "<div><span id=\"outer\" style=\"display:inline;background:#111111\">aa <em id=\"inner\" style=\"display:inline;background:#222222\">bb <strong id=\"strong\" style=\"display:inline;background:#333333\">cc dd </strong>ee ff </em>gg</span></div>";
    let output = render_html(source, options()).unwrap();
    let inner = node_with_id(&output.document, "inner");
    let strong = node_with_id(&output.document, "strong");

    let inner_fragments = fragments_for_dom(&output.layout.fragments, inner);
    assert!(inner_fragments.len() >= 2);
    assert!(!fragments_for_dom(&output.layout.fragments, strong).is_empty());
    assert!(output.display_list.has_unique_ids());
    assert!(output.display_list.validate().is_ok());
}

#[test]
fn pure_inline_subtree_style_change_matches_fresh_render() {
    let source = "<div><span id=\"outer\" style=\"display:inline;background:#111111\">aa <em id=\"inner\" style=\"display:inline;background:#222222\">bb <strong style=\"display:inline\">cc dd </strong>ee ff </em>gg</span></div>";
    let expected = "<div><span id=\"outer\" style=\"display:inline;background:#111111\">aa <em id=\"inner\" style=\"display:inline;background:#778899\">bb <strong style=\"display:inline\">cc dd </strong>ee ff </em>gg</span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let inner = node_with_id(session.document(), "inner");

    session
        .document_mut()
        .set_attribute(inner, "style", "display:inline;background:#778899")
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::FullRebuild);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
    assert_eq!(
        session.layout().fragments.snapshot(),
        fresh.layout.fragments.snapshot()
    );
    assert_eq!(
        session.display_list().snapshot(),
        fresh.display_list.snapshot()
    );
}
