from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:120]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


engine = "crates/rarog-engine/src/lib.rs"
replace(
    engine,
    """        let mut requires_full_rebuild = mutation_history_lost;
        for mutation in &mutations {
            match mutation {
                MutationKind::Attribute { node, name }
                    if matches!(name.as_str(), "id" | "class" | "style") =>
                {
                    style_candidates.insert(*node);
                }
                MutationKind::Attribute { .. } => {}
                MutationKind::NodeCreated { .. }
                | MutationKind::ChildAdded { .. }
                | MutationKind::Reparented { .. }
                | MutationKind::CharacterData { .. } => {
                    requires_full_rebuild = true;
                }
            }
        }

        let new_styles = StyleSet::for_document(&self.document);""",
    """        let mut requires_full_rebuild = mutation_history_lost;
        let mut stylesheet_sources_changed = mutation_history_lost;
        for mutation in &mutations {
            match mutation {
                MutationKind::Attribute { node, name }
                    if matches!(name.as_str(), "id" | "class" | "style") =>
                {
                    style_candidates.insert(*node);
                }
                MutationKind::Attribute { .. } => {}
                MutationKind::NodeCreated { .. }
                | MutationKind::ChildAdded { .. }
                | MutationKind::Reparented { .. } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed = true;
                }
                MutationKind::CharacterData { node } => {
                    requires_full_rebuild = true;
                    stylesheet_sources_changed |= node_is_within_style_element(&self.document, *node);
                }
            }
        }

        let new_styles = if stylesheet_sources_changed {
            StyleSet::for_document(&self.document)
        } else {
            self.styles.clone()
        };
        validate_style_limits(&new_styles, self.limits)?;""",
)
replace(
    engine,
    """fn layout_style_for_dom(node: &LayoutNode, dom_node: NodeId) -> Option<ComputedStyle> {""",
    """fn node_is_within_style_element(document: &Document, mut node: NodeId) -> bool {
    while let Some(current) = document.node(node) {
        if let NodeKind::Element(element) = &current.kind
            && element.tag_name.as_str() == "style"
        {
            return true;
        }
        let Some(parent) = current.parent else {
            return false;
        };
        node = parent;
    }
    false
}

fn layout_style_for_dom(node: &LayoutNode, dom_node: NodeId) -> Option<ComputedStyle> {""",
)

correctness = "crates/rarog-engine/tests/r01_correctness.rs"
replace(
    correctness,
    """use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};""",
    """use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use rarog_engine::{
    IncrementalMode, RenderError, RenderLimits, RenderOptions, RenderSession, render_html,
    render_html_with_limits,
};""",
)
marker = """#[test]
fn non_finite_viewport_is_rejected() {"""
text = Path(correctness).read_text(encoding="utf-8")
if marker not in text:
    raise SystemExit("correctness marker missing")
tests = r'''#[test]
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
    let mut session = RenderSession::new_with_limits(
        "<div id=\"target\"></div>",
        options(),
        limits,
    )
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
    let source = "<style id=\"sheet\">#target { background:#112233; }</style><div id=\"target\">x</div>";
    let expected = "<style id=\"sheet\">#target { background:#445566; }</style><div id=\"target\">x</div>";
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
        let style = format!(
            "width:{width}px;height:20px;background:#{shade:02x}{shade:02x}{shade:02x}"
        );
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
        assert_eq!(
            session.layout().fragments.snapshot(),
            fresh.layout.fragments.snapshot()
        );
        assert_eq!(session.display_list().snapshot(), fresh.display_list.snapshot());
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

'''
Path(correctness).write_text(text.replace(marker, tests + marker), encoding="utf-8")

paint = "crates/rarog-paint/src/lib.rs"
text = Path(paint).read_text(encoding="utf-8")
marker = """    #[test]
    fn opacity_composes_across_nested_scopes() {"""
if marker not in text:
    raise SystemExit("paint transform test marker missing")
test = """    #[test]
    fn nested_transform_order_follows_display_list_push_order() {
        let rect = Rect::new(1.0, 1.0, 2.0, 2.0);
        let transformed = transform_rect(
            rect,
            &[
                Transform2D::translation(10.0, 0.0),
                Transform2D::scale(2.0, 3.0),
            ],
        );
        assert_eq!(transformed, Rect::new(22.0, 3.0, 4.0, 6.0));
    }

"""
Path(paint).write_text(text.replace(marker, test + marker), encoding="utf-8")
