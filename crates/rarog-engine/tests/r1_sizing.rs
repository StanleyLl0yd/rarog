use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_types::{Color, Size};

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 220.0,
            height: 160.0,
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
fn max_width_incremental_update_matches_a_fresh_render() {
    let source = "<div id=\"hero\" style=\"width:auto;max-width:140px;height:20px;background:#112233\"></div>";
    let expected_source = "<div id=\"hero\" style=\"width:auto;max-width:90px;height:20px;background:#112233\"></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let hero = element_with_id(session.document(), "hero");

    session
        .document_mut()
        .set_attribute(
            hero,
            "style",
            "width:auto;max-width:90px;height:20px;background:#112233",
        )
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected_source, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::SubtreeRelayout);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}

#[test]
fn changing_flow_root_boundary_falls_back_to_a_correct_full_rebuild() {
    let source = "<div><div id=\"box\" style=\"display:block;margin-top:10px\"><div style=\"height:20px;margin-top:30px;background:#112233\"></div></div></div>";
    let expected_source = "<div><div id=\"box\" style=\"display:flow-root;margin-top:10px\"><div style=\"height:20px;margin-top:30px;background:#112233\"></div></div></div>";
    let mut session = RenderSession::new(source, options()).unwrap();
    let node = element_with_id(session.document(), "box");

    session
        .document_mut()
        .set_attribute(node, "style", "display:flow-root;margin-top:10px")
        .unwrap();
    let report = session.update().unwrap();
    let fresh = render_html(expected_source, options()).unwrap();

    assert_eq!(report.mode, IncrementalMode::FullRebuild);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
