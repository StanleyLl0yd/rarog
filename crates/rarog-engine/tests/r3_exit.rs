use rarog_compositor::{FrameCause, FrameScheduler};
use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{RenderOptions, RenderOutput, render_html};
use rarog_layout::Fragment;
use rarog_resources::{
    DecodedImage, ImageDecodeOutcome, ImageDecodeQueue, ImageResourceStatus, ImageResourceStore,
};
use rarog_scroll::ScrollTree;
use rarog_types::{Color, Point, Rect, Size};

const R3_BACKLOG: &str = include_str!("../../../docs/R3-BACKLOG.md");
const PRE_R4_AUDIT: &str = include_str!("../../../docs/PRE-R4-AUDIT.md");

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 320.0,
            height: 200.0,
        },
        background: Color::WHITE,
    }
}

fn node_with_id(document: &Document, id: &str) -> NodeId {
    fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
        if document.node(node).is_some_and(|node| {
            matches!(
                &node.kind,
                NodeKind::Element(element)
                    if element.attributes.get("id").map(String::as_str) == Some(id)
            )
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

fn fragment_for_node(fragment: &Fragment, node: NodeId) -> Option<&Fragment> {
    if fragment.dom_node == Some(node) {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| fragment_for_node(child, node))
}

fn fragment_with_id<'a>(output: &'a RenderOutput, id: &str) -> &'a Fragment {
    let node = node_with_id(&output.document, id);
    fragment_for_node(&output.layout.fragments.root, node).expect("fixture node creates a fragment")
}

#[test]
fn r3_exit_manifest_is_complete_and_pre_r4_gate_is_pending() {
    assert!(R3_BACKLOG.contains("Status: **complete**."));
    assert!(
        !R3_BACKLOG
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]")),
        "R3 backlog contains an unchecked milestone item"
    );
    assert!(PRE_R4_AUDIT.contains("Status: **pending**."));
    assert!(PRE_R4_AUDIT.contains("R4 implementation is blocked"));
}

#[test]
fn r3_exit_css_grid_exposes_fraction_and_content_sized_tracks() {
    let fractional = render_html(
        r#"<div id="grid" style="display:grid;width:110px;justify-content:flex-start;grid-template-columns:1fr 3fr;grid-template-rows:20px;column-gap:10px"><div id="a"></div><div id="b"></div></div>"#,
        options(),
    )
    .expect("R3 fractional Grid fixture must render");

    let grid = fragment_with_id(&fractional, "grid");
    let a = fragment_with_id(&fractional, "a");
    let b = fragment_with_id(&fractional, "b");
    assert_eq!(a.boxes.border_box.size.width, 25.0);
    assert_eq!(
        b.boxes.border_box.origin.x - grid.boxes.content_box.origin.x,
        35.0
    );
    assert_eq!(b.boxes.border_box.size.width, 75.0);

    let intrinsic = render_html(
        r#"<div id="grid" style="display:grid;width:200px;justify-content:flex-start;grid-template-columns:min-content max-content;grid-template-rows:20px"><div id="min">hello world</div><div id="max">hello world</div></div>"#,
        options(),
    )
    .expect("R3 content-sized Grid fixture must render");

    let min = fragment_with_id(&intrinsic, "min");
    let max = fragment_with_id(&intrinsic, "max");
    assert_eq!(min.boxes.border_box.size.width, 40.0);
    assert_eq!(max.boxes.border_box.size.width, 88.0);
}

#[test]
fn r3_exit_frame_scheduler_coalesces_and_retries_pending_causes() {
    let mut scheduler = FrameScheduler::new();
    scheduler.request(FrameCause::Scroll);
    scheduler.request(FrameCause::ResourceReady);

    let first = scheduler
        .begin()
        .expect("frame scheduler begin must succeed")
        .expect("coalesced causes request a frame");
    assert!(first.reasons().contains(FrameCause::Scroll));
    assert!(first.reasons().contains(FrameCause::ResourceReady));
    assert_eq!(first.primary_cause(), FrameCause::ResourceReady);

    scheduler
        .discard(first.id())
        .expect("discard returns causes to pending state");
    let retry = scheduler
        .begin()
        .expect("retry begin must succeed")
        .expect("discarded causes remain pending");
    assert_eq!(retry.reasons(), first.reasons());
    scheduler
        .complete(retry.id())
        .expect("retry completion must succeed");
    assert!(scheduler.begin().unwrap().is_none());
}

#[test]
fn r3_exit_bounded_image_decode_lifecycle_reaches_ready_state() {
    let mut store = ImageResourceStore::default();
    let mut queue = ImageDecodeQueue::default();

    let reservation = queue
        .enqueue(&mut store, vec![1, 2, 3, 4])
        .expect("bounded encoded resource must enqueue");
    assert_eq!(
        store.status(reservation.resource()),
        Some(ImageResourceStatus::Pending)
    );
    assert_eq!(queue.retained_encoded_bytes(), 4);

    let work = queue
        .begin_next()
        .expect("decode begin must succeed")
        .expect("queued resource produces work");
    assert_eq!(work.request(), reservation.request());
    assert_eq!(work.resource(), reservation.resource());
    assert_eq!(work.encoded(), &[1, 2, 3, 4]);

    let image = DecodedImage::try_new(1, 1, vec![Color::WHITE])
        .expect("single-pixel decoded image is valid");
    let reference = queue
        .complete(&mut store, work.request(), ImageDecodeOutcome::Ready(image))
        .expect("decode completion must succeed")
        .expect("ready decode returns a resource reference");

    assert_eq!(reference.id(), reservation.resource());
    assert_eq!(
        store.status(reservation.resource()),
        Some(ImageResourceStatus::Ready)
    );
    assert_eq!(queue.retained_encoded_bytes(), 0);
}

#[test]
fn r3_exit_scroll_tree_clamps_offsets_and_reports_damage() {
    let viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
    let mut tree = ScrollTree::with_defaults(
        viewport,
        Size {
            width: 300.0,
            height: 240.0,
        },
    )
    .expect("R3 scroll fixture must construct");
    let root = tree.root();

    let delta = tree
        .scroll_by(root, Point { x: 500.0, y: 500.0 })
        .expect("scroll must succeed");

    assert!(delta.changed());
    assert_eq!(delta.previous, Point::default());
    assert_eq!(delta.current, Point { x: 200.0, y: 160.0 });
    assert_eq!(delta.damage, Some(viewport));
    assert_eq!(
        tree.snapshot(root).expect("root snapshot exists").offset,
        delta.current
    );
}
