use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_types::{Color, Size};

const R1_BACKLOG: &str = include_str!("../../../docs/R1-BACKLOG.md");

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 96.0,
            height: 120.0,
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
fn r1_exit_manifest_has_no_open_items() {
    assert!(R1_BACKLOG.contains("Status: **complete**."));
    assert!(
        !R1_BACKLOG
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]")),
        "R1 backlog contains an unchecked milestone item; move later work to ROADMAP.md or complete the Flame requirement"
    );
}

#[test]
fn r1_exit_retained_mixed_update_matches_fresh_render() {
    let source = "<div id=\"target\" style=\"width:48px;background:#112233\">one</div>";
    let expected_source =
        "<div id=\"target\" style=\"width:72px;background:#778899\">one two three four</div>";
    let mut session = RenderSession::new(source, options()).expect("R1 fixture must render");
    let target = node_with_id(session.document(), "target");
    let text = session.document().children(target).unwrap()[0];

    session
        .document_mut()
        .set_text(text, "one two three four")
        .expect("text mutation must succeed");
    session
        .document_mut()
        .set_attribute(target, "style", "width:72px;background:#778899")
        .expect("style mutation must succeed");

    let report = session.update().expect("retained R1 update must succeed");
    let fresh = render_html(expected_source, options()).expect("fresh R1 fixture must render");

    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert!(report.retained_display_list);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}
