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
fn vertical_align_change_uses_correct_full_rebuild() {
    let source = "<div>a<span id=\"chip\" style=\"display:inline;width:20px;height:10px;background:#112233\"></span></div>";
    let expected_source = "<div>a<span id=\"chip\" style=\"display:inline;width:20px;height:10px;background:#112233;vertical-align:bottom\"></span></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let chip = element_with_id(session.document(), "chip");

    session
        .document_mut()
        .set_attribute(
            chip,
            "style",
            "display:inline;width:20px;height:10px;background:#112233;vertical-align:bottom",
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
