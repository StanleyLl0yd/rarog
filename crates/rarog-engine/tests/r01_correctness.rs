use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_engine::{
    IncrementalMode, RenderError, RenderLimits, RenderOptions, RenderSession, render_html,
    render_html_with_limits,
};
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
        if let Some(dom_node) = document.node(node) {
            if let NodeKind::Element(element) = &dom_node.kind {
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

fn update_style(source: &str, style: &str) -> (RenderSession, IncrementalMode) {
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");
    session
        .document_mut()
        .set_attribute(target, "style", style)
        .expect("style mutation succeeds");
    let mode = session.update().expect("incremental update succeeds").mode;
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
fn vertical_flow_append_and_reparent_reflow_are_preserved() {
    let source = r#"<div id="target" style="height:20px;background:#112233"></div><div style="height:10px;background:#445566"></div>"#;
    let expected = r#"<div id="target" style="height:32px;background:#112233"></div><div style="height:10px;background:#445566"></div>"#;
    let (flow, flow_mode) = update_style(source, "height:32px;background:#112233");
    assert_eq!(flow_mode, IncrementalMode::FlowRelayout);
    assert_matches_fresh(&flow, expected);

    let mut structural = RenderSession::new(source, options()).expect("session starts");
    let target = element_with_id(structural.document(), "target");
    structural
        .document_mut()
        .append_new(target, NodeKind::Text("!".into()))
        .expect("append mutation succeeds");
    let append_report = structural.update().expect("append update succeeds");
    assert_eq!(append_report.mode, IncrementalMode::FlowRelayout);
    assert!(append_report.retained_display_list);
    assert!(!append_report.styles_rebuilt);
    assert_matches_fresh(
        &structural,
        r#"<div id="target" style="height:20px;background:#112233">!</div><div style="height:10px;background:#445566"></div>"#,
    );

    let reparent_source =
        r#"<div id="from"><span id="child">Rarog</span></div><div id="to"></div>"#;
    let reparent_expected =
        r#"<div id="from"></div><div id="to"><span id="child">Rarog</span></div>"#;
    let mut reparent = RenderSession::new(reparent_source, options()).expect("session starts");
    let child = element_with_id(reparent.document(), "child");
    let destination = element_with_id(reparent.document(), "to");
    reparent
        .document_mut()
        .append_child(destination, child)
        .expect("reparent mutation succeeds");
    let reparent_report = reparent.update().expect("reparent update succeeds");
    assert_eq!(reparent_report.mode, IncrementalMode::FlowRelayout);
    assert!(reparent_report.retained_display_list);
    assert!(!reparent_report.styles_rebuilt);
    assert_matches_fresh(&reparent, reparent_expected);
}

#[test]
fn structural_limits_reject_deep_and_wide_documents_before_recursive_rendering() {
    let deep = format!("{}x{}", "<div>".repeat(16), "</div>".repeat(16));
    let depth_limits = RenderLimits {
        max_dom_depth: 8,
        ..RenderLimits::default()
    };
    assert!(matches!(
        render_html_with_limits(&deep, options(), depth_limits),
        Err(RenderError::DomDepthLimitExceeded { .. })
    ));

    let wide = "<div>x</div>".repeat(32);
    let node_limits = RenderLimits {
        max_dom_nodes: 16,
        ..RenderLimits::default()
    };
    assert!(matches!(
        render_html_with_limits(&wide, options(), node_limits),
        Err(RenderError::DomNodeLimitExceeded { .. })
    ));
}

#[test]
fn mutation_growth_is_rejected_before_incremental_recursive_work() {
    let limits = RenderLimits {
        max_dom_depth: 6,
        ..RenderLimits::default()
    };
    let mut session =
        RenderSession::new_with_limits("<div id=\"target\"></div>", options(), limits)
            .expect("session starts");
    let mut parent = element_with_id(session.document(), "target");
    for _ in 0..8 {
        parent = session
            .document_mut()
            .append_new(parent, NodeKind::Element(ElementData::html("div")))
            .expect("fixture mutation succeeds");
    }

    assert!(matches!(
        session.update(),
        Err(RenderError::DomDepthLimitExceeded { .. })
    ));
}

#[test]
fn detached_mutations_do_not_force_connected_render_work() {
    let mut session = RenderSession::new(
        "<div id=\"target\" style=\"background:#112233\">x</div>",
        options(),
    )
    .expect("session starts");
    let before = session.framebuffer().stable_hash64();
    let detached = session
        .document_mut()
        .create_node(NodeKind::Element(ElementData::html("section")))
        .expect("detached node is created");
    session
        .document_mut()
        .set_attribute(detached, "class", "unused")
        .expect("detached mutation succeeds");

    let report = session.update().expect("detached update succeeds");
    assert_eq!(report.mode, IncrementalMode::Unchanged);
    assert_eq!(session.framebuffer().stable_hash64(), before);
}

#[test]
fn stylesheet_text_mutation_rebuilds_stylesheet_sources() {
    let source =
        "<style id=\"sheet\">#target { background:#112233; }</style><div id=\"target\">x</div>";
    let expected =
        "<style id=\"sheet\">#target { background:#445566; }</style><div id=\"target\">x</div>";
    let mut session = RenderSession::new(source, options()).expect("session starts");
    let sheet = element_with_id(session.document(), "sheet");
    let text = *session
        .document()
        .children(sheet)
        .and_then(|children| children.first())
        .expect("style element contains text");
    session
        .document_mut()
        .set_text(text, "#target { background:#445566; }")
        .expect("stylesheet text mutation succeeds");

    assert_eq!(
        session.update().expect("stylesheet update succeeds").mode,
        IncrementalMode::FullRebuild
    );
    assert_matches_fresh(&session, expected);
}

#[test]
fn deterministic_incremental_sequence_matches_fresh_render() {
    let base = "<div id=\"target\" style=\"width:80px;height:20px;background:#000000\">Rarog</div>";
    let mut session = RenderSession::new(base, options()).expect("session starts");
    let target = element_with_id(session.document(), "target");

    for step in 0..24u32 {
        let width = 80 + (step % 5) * 4;
        let shade = (step * 17) & 0xff;
        let style =
            format!("width:{width}px;height:20px;background:#{shade:02x}{shade:02x}{shade:02x}");
        session
            .document_mut()
            .set_attribute(target, "style", style.clone())
            .expect("mutation succeeds");
        session.update().expect("incremental update succeeds");

        let fresh_source = format!("<div id=\"target\" style=\"{style}\">Rarog</div>");
        let fresh = render_html(&fresh_source, options()).expect("fresh render succeeds");
        assert_eq!(
            session.framebuffer().stable_hash64(),
            fresh.framebuffer.stable_hash64()
        );
        assert_eq!(session.styles().snapshot(), fresh.styles.snapshot());
        assert_eq!(
            session.layout().tree.snapshot(),
            fresh.layout.tree.snapshot()
        );
        assert_eq!(session.display_list().len(), fresh.display_list.len());
    }
}

#[test]
fn malformed_bootstrap_corpus_does_not_panic() {
    for source in [
        "",
        "<",
        "<div",
        "</div>",
        "<style>{</style>",
        "<div><span></div>",
        "<div style=\"width:NaNpx\">x</div>",
    ] {
        let _ = render_html_with_limits(source, options(), RenderLimits::default());
    }
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
