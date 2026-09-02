use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_paint::DisplayCommand;
use rarog_types::{Color, Size};

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

fn has_text_color(session: &RenderSession, color: Color) -> bool {
    session.display_list().commands().iter().any(|command| {
        matches!(command, DisplayCommand::TextPlaceholder { color: actual, .. } if *actual == color)
    })
}

#[test]
fn inherited_color_reaches_paint_and_updates_incrementally() {
    let options = RenderOptions {
        viewport: Size {
            width: 160.0,
            height: 90.0,
        },
        background: Color::WHITE,
    };
    let source = "<div id=\"parent\" style=\"color:#112233\"><span>Rarog</span></div>";
    let expected = "<div id=\"parent\" style=\"color:#445566\"><span>Rarog</span></div>";
    let mut session = RenderSession::new(source, options).unwrap();
    assert!(has_text_color(&session, Color::rgb(0x11, 0x22, 0x33)));

    let parent = element_with_id(session.document(), "parent");
    session
        .document_mut()
        .set_attribute(parent, "style", "color:#445566")
        .unwrap();
    let report = session.update().unwrap();

    assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
    assert!(has_text_color(&session, Color::rgb(0x44, 0x55, 0x66)));
    let fresh = render_html(expected, options).unwrap();
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
