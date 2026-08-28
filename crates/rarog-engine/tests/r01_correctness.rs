use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use rarog_types::{Color, Size};

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
        if let Some(dom_node) = document.node(node)
            && let NodeKind::Element(element) = &dom_node.kind
            && element.attributes.get("id").map(String::as_str) == Some(id)
        {
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

fn update_style(source: &str, style: &str) -> (RenderSession, IncrementalMode) {
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");
    session
        .document_mut()
        .set_attribute(target, "style", style)
        .expect("style mutation succeeds");
    let mode = session.update().mode;
    (session, mode)
}

fn assert_matches_fresh(session: &RenderSession, source: &str) {
    let fresh = render_html(source, options()).expect("fresh render succeeds");
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}

#[test]
fn deterministic_full_render_contract() {
    let source = "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>";
    let first = render_html(source, options()).expect("fixture renders");
    let second = render_html(source, options()).expect("fixture renders repeatedly");

    assert_eq!(first.document.snapshot(), second.document.snapshot());
    assert_eq!(first.styles.snapshot(), second.styles.snapshot());
    assert_eq!(first.layout.tree.snapshot(), second.layout.tree.snapshot());
    assert_eq!(
        first.layout.fragments.snapshot(),
        second.layout.fragments.snapshot()
    );
    assert_eq!(
        first.display_list.snapshot(),
        second.display_list.snapshot()
    );
    assert_eq!(
        first.deterministic_signature_hash(),
        second.deterministic_signature_hash()
    );
}

#[test]
fn incremental_paths_match_fresh_render() {
    let base = "<div id=\"target\" style=\"width:80px;height:20px;background:#112233\">Rarog</div>";

    let paint_style = "width:80px;height:20px;background:#445566";
    let paint_source =
        "<div id=\"target\" style=\"width:80px;height:20px;background:#445566\">Rarog</div>";
    let (paint, paint_mode) = update_style(base, paint_style);
    assert_eq!(paint_mode, IncrementalMode::PaintOnlyReuse);
    assert_matches_fresh(&paint, paint_source);

    let geometry_style = "width:96px;height:20px;background:#445566";
    let geometry_source =
        "<div id=\"target\" style=\"width:96px;height:20px;background:#445566\">Rarog</div>";
    let (geometry, geometry_mode) = update_style(base, geometry_style);
    assert_eq!(geometry_mode, IncrementalMode::SubtreeRelayout);
    assert_matches_fresh(&geometry, geometry_source);
}

#[test]
fn vertical_flow_and_structural_fallback_are_preserved() {
    let source = "<div id=\"target\" style=\"height:20px;background:#112233\"></div><div style=\"height:10px;background:#445566\"></div>";
    let expected = "<div id=\"target\" style=\"height:32px;background:#112233\"></div><div style=\"height:10px;background:#445566\"></div>";
    let (flow, flow_mode) = update_style(source, "height:32px;background:#112233");
    assert_eq!(flow_mode, IncrementalMode::FlowRelayout);
    assert_matches_fresh(&flow, expected);

    let mut structural = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(structural.document(), "target");
    structural
        .document_mut()
        .append_new(target, NodeKind::Text("!".into()))
        .expect("structural mutation succeeds");
    assert_eq!(structural.update().mode, IncrementalMode::FullRebuild);
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
