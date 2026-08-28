use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_types::{Color, Size};

const FIXTURE: &str = "<style>.card { width:80px; padding:4px; background:#112233; } #hero { border-width:2px; border-color:#000000; }</style><div id=\"hero\" class=\"card\">Rarog</div>";

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 160.0,
            height: 90.0,
        },
        background: Color::WHITE,
    }
}

fn element_with_id(document: &Document, id: &str) -> NodeId {
    fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
        if let NodeKind::Element(element) = &document.node(node).kind
            && element.attributes.get("id").map(String::as_str) == Some(id)
        {
            return Some(node);
        }
        document
            .children(node)
            .iter()
            .find_map(|child| find(document, *child, id))
    }

    find(document, document.root(), id).expect("fixture contains requested id")
}

#[test]
fn deterministic_full_render_contract() {
    let first = render_html(FIXTURE, options()).expect("fixture renders");
    let second = render_html(FIXTURE, options()).expect("fixture renders repeatedly");

    assert_eq!(first.document.snapshot(), second.document.snapshot());
    assert_eq!(first.styles.snapshot(), second.styles.snapshot());
    assert_eq!(first.layout.tree.snapshot(), second.layout.tree.snapshot());
    assert_eq!(
        first.layout.fragments.snapshot(),
        second.layout.fragments.snapshot()
    );
    assert_eq!(first.display_list.snapshot(), second.display_list.snapshot());
    assert_eq!(
        first.framebuffer.stable_hash64(),
        second.framebuffer.stable_hash64()
    );
    assert_eq!(
        first.deterministic_signature_hash(),
        second.deterministic_signature_hash()
    );
}

#[test]
fn paint_only_update_matches_fresh_render() {
    let source = "<div id=\"target\" style=\"width:80px;height:20px;background:#112233\">Rarog</div>";
    let expected_source = "<div id=\"target\" style=\"width:80px;height:20px;background:#445566\">Rarog</div>";
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");

    session
        .document_mut()
        .set_attribute(
            target,
            "style",
            "width:80px;height:20px;background:#445566",
        )
        .expect("style mutation succeeds");
    let report = session.update();
    let expected = render_html(expected_source, options()).expect("expected fixture renders");

    assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        expected.framebuffer.stable_hash64()
    );
}

#[test]
fn subtree_relayout_matches_fresh_render() {
    let source = "<div id=\"target\" style=\"width:80px;height:20px;background:#112233\">Rarog</div>";
    let expected_source = "<div id=\"target\" style=\"width:96px;height:20px;background:#445566\">Rarog</div>";
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");

    session
        .document_mut()
        .set_attribute(
            target,
            "style",
            "width:96px;height:20px;background:#445566",
        )
        .expect("style mutation succeeds");
    let report = session.update();
    let expected = render_html(expected_source, options()).expect("expected fixture renders");

    assert_eq!(report.mode, IncrementalMode::SubtreeRelayout);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        expected.framebuffer.stable_hash64()
    );
}

#[test]
fn flow_relayout_matches_fresh_render() {
    let source = "<div id=\"target\" style=\"height:20px;background:#112233\"></div><div style=\"height:10px;background:#445566\"></div>";
    let expected_source = "<div id=\"target\" style=\"height:32px;background:#112233\"></div><div style=\"height:10px;background:#445566\"></div>";
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");

    session
        .document_mut()
        .set_attribute(target, "style", "height:32px;background:#112233")
        .expect("style mutation succeeds");
    let report = session.update();
    let expected = render_html(expected_source, options()).expect("expected fixture renders");

    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        expected.framebuffer.stable_hash64()
    );
}

#[test]
fn structural_change_uses_full_rebuild_fallback() {
    let source = "<div id=\"target\" style=\"height:20px\">Rarog</div>";
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");

    session
        .document_mut()
        .append_new(target, NodeKind::Text("!".into()))
        .expect("structural mutation succeeds");

    assert_eq!(session.update().mode, IncrementalMode::FullRebuild);
}

#[test]
fn non_finite_viewport_is_rejected() {
    let result = render_html(
        "<div>Rarog</div>",
        RenderOptions {
            viewport: Size {
                width: f32::NAN,
                height: 90.0,
            },
            background: Color::WHITE,
        },
    );

    assert!(result.is_err());
}
