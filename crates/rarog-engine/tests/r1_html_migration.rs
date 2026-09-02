use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{
    RenderError, RenderLimits, RenderOptions, render_html, render_html_with_limits,
};

fn find_element(document: &Document, tag_name: &str) -> Option<NodeId> {
    let mut stack = vec![document.root()];
    while let Some(node) = stack.pop() {
        let Some(current) = document.node(node) else {
            continue;
        };
        if let NodeKind::Element(element) = &current.kind {
            if element.tag_name.as_str() == tag_name {
                return Some(node);
            }
        }
        stack.extend(current.children.iter().rev().copied());
    }
    None
}

#[test]
fn engine_default_uses_standards_tree_builder() {
    let output = render_html(
        "<table><tr><td>x</td></tr></table>",
        RenderOptions::default(),
    )
    .unwrap();

    assert!(find_element(&output.document, "tbody").is_some());
}

#[test]
fn standards_path_rejects_dom_depth_before_layout() {
    let limits = RenderLimits {
        max_dom_depth: 8,
        ..RenderLimits::default()
    };
    let source = format!("{}x{}", "<div>".repeat(16), "</div>".repeat(16));

    assert!(matches!(
        render_html_with_limits(&source, RenderOptions::default(), limits),
        Err(RenderError::DomDepthLimitExceeded { .. })
    ));
}

#[test]
fn standards_path_rejects_dom_node_budget_before_layout() {
    let limits = RenderLimits {
        max_dom_nodes: 12,
        ..RenderLimits::default()
    };
    let source = format!("<div>{}</div>", "<span>x</span>".repeat(20));

    assert!(matches!(
        render_html_with_limits(&source, RenderOptions::default(), limits),
        Err(RenderError::DomNodeLimitExceeded { .. })
    ));
}

#[test]
fn malformed_html_does_not_panic_at_render_boundary() {
    let result = std::panic::catch_unwind(|| {
        render_html(
            "<table><b><tr><td>&notanentity;<svg><foreignObject><p>x",
            RenderOptions::default(),
        )
    });

    assert!(result.is_ok());
}
