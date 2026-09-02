use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession};
use rarog_layout::fragment_for_dom;
use rarog_types::Size;

fn element_with_id(document: &Document, id: &str) -> NodeId {
    let mut stack = vec![document.root()];
    while let Some(node) = stack.pop() {
        if let Some(NodeKind::Element(element)) = document.node(node).map(|node| &node.kind) {
            if element
                .attributes
                .get("id")
                .is_some_and(|value| value == id)
            {
                return node;
            }
        }
        if let Some(children) = document.children(node) {
            stack.extend(children.iter().rev().copied());
        }
    }
    panic!("fixture element not found: {id}");
}

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 240.0,
            height: 240.0,
        },
        ..RenderOptions::default()
    }
}

#[test]
fn margin_collapse_updates_match_a_fresh_render() {
    let initial = "<div id='a' style='height:20px;margin-bottom:10px;background:#112233'></div><div id='b' style='height:20px;margin-top:30px;background:#445566'></div>";
    let updated = "<div id='a' style='height:20px;margin-bottom:50px;background:#112233'></div><div id='b' style='height:20px;margin-top:30px;background:#445566'></div>";
    let mut session = RenderSession::new(initial, options()).unwrap();
    let first = element_with_id(session.document(), "a");
    let second = element_with_id(session.document(), "b");

    session
        .document_mut()
        .set_attribute(
            first,
            "style",
            "height:20px;margin-bottom:50px;background:#112233",
        )
        .unwrap();
    let report = session.update().unwrap();
    assert_eq!(report.mode, IncrementalMode::FlowRelayout);

    let incremental_y = fragment_for_dom(&session.layout().fragments, second)
        .unwrap()
        .boxes
        .border_box
        .origin
        .y;

    let fresh = RenderSession::new(updated, options()).unwrap();
    let fresh_second = element_with_id(fresh.document(), "b");
    let fresh_y = fragment_for_dom(&fresh.layout().fragments, fresh_second)
        .unwrap()
        .boxes
        .border_box
        .origin
        .y;

    assert_eq!(incremental_y, fresh_y);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer().stable_hash64()
    );
}
