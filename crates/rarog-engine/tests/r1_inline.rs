use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_types::{Color, Size};

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 120.0,
            height: 100.0,
        },
        background: Color::WHITE,
    }
}

fn element_with_id(document: &Document, id: &str) -> NodeId {
    fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
        if let Some(current) = document.node(node) {
            if let NodeKind::Element(element) = &current.kind {
                if element.attributes.get("id").map(String::as_str) == Some(id) {
                    return Some(node);
                }
            }
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
fn inline_geometry_change_uses_correct_full_rebuild() {
    let source = "<div><span id=\"chip\" style=\"display:inline;width:40px;height:10px;background:#112233\"></span><span style=\"display:inline;width:60px;height:10px;background:#445566\"></span></div>";
    let expected_source = "<div><span id=\"chip\" style=\"display:inline;width:70px;height:10px;background:#112233\"></span><span style=\"display:inline;width:60px;height:10px;background:#445566\"></span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let chip = element_with_id(session.document(), "chip");

    session
        .document_mut()
        .set_attribute(
            chip,
            "style",
            "display:inline;width:70px;height:10px;background:#112233",
        )
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected_source, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::FullRebuild);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}

#[test]
fn inline_paint_only_change_keeps_retained_path() {
    let source = "<div><span id=\"chip\" style=\"display:inline;width:40px;height:10px;background:#112233\"></span></div>";
    let expected_source = "<div><span id=\"chip\" style=\"display:inline;width:40px;height:10px;background:#778899\"></span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let chip = element_with_id(session.document(), "chip");

    session
        .document_mut()
        .set_attribute(
            chip,
            "style",
            "display:inline;width:40px;height:10px;background:#778899",
        )
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected_source, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
