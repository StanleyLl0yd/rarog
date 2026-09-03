use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{
    EngineEventLoop, EngineEventLoopStep, IncrementalMode, RenderOptions, RenderSession, render_html,
};
use rarog_platform::{PlatformCapabilities, PlatformService};
use rarog_scheduler::{SchedulerLimits, TaskSource, WorkId};
use rarog_types::{Color, Size};

const R2_BACKLOG: &str = include_str!("../../../docs/R2-BACKLOG.md");

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 120.0,
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
fn r2_exit_manifest_has_no_open_items() {
    assert!(R2_BACKLOG.contains("Status: **complete**."));
    assert!(
        !R2_BACKLOG
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]")),
        "R2 backlog contains an unchecked milestone item; move later work to ROADMAP.md or complete the Flight requirement"
    );
}

#[test]
fn r2_exit_task_and_microtask_mutations_render_at_checkpoint() {
    let source = "<div id=\"target\" style=\"width:48px;background:#112233\">one</div>";
    let expected_source =
        "<div id=\"target\" style=\"width:72px;background:#778899\">one two three four</div>";
    let mut session = RenderSession::new(source, options()).expect("R2 fixture must render");
    let target = node_with_id(session.document(), "target");
    let text = session.document().children(target).unwrap()[0];
    let before = session.framebuffer().stable_hash64();

    let limits = SchedulerLimits::try_new(8, 8).unwrap();
    let mut event_loop = EngineEventLoop::<(), ()>::new(limits).unwrap();
    let task_id = event_loop
        .queue_task(TaskSource::DomManipulation, ())
        .unwrap();

    let Some(EngineEventLoopStep::Task(task)) = event_loop.next_step(&mut session).unwrap() else {
        panic!("expected DOM task");
    };
    assert_eq!(task.id, task_id);
    session
        .document_mut()
        .set_text(text, "one two three four")
        .expect("task mutation must succeed");
    event_loop.queue_microtask(()).unwrap();
    event_loop.complete(WorkId::Task(task.id)).unwrap();
    assert_eq!(session.framebuffer().stable_hash64(), before);

    let Some(EngineEventLoopStep::Microtask(microtask)) =
        event_loop.next_step(&mut session).unwrap()
    else {
        panic!("expected microtask");
    };
    session
        .document_mut()
        .set_attribute(target, "style", "width:72px;background:#778899")
        .expect("microtask mutation must succeed");
    event_loop
        .complete(WorkId::Microtask(microtask.id))
        .unwrap();
    assert_eq!(session.framebuffer().stable_hash64(), before);

    let Some(EngineEventLoopStep::RenderCheckpoint(report)) =
        event_loop.next_step(&mut session).unwrap()
    else {
        panic!("expected render checkpoint");
    };
    let fresh = render_html(expected_source, options()).expect("fresh R2 fixture must render");

    assert_ne!(report.mode, IncrementalMode::FullRebuild);
    assert!(report.retained_display_list);
    assert_eq!(
        session.framebuffer().stable_hash64(),
        fresh.framebuffer.stable_hash64()
    );
}

#[test]
fn r2_exit_platform_capabilities_keep_input_ime_and_clipboard_distinct() {
    let input_only = PlatformCapabilities {
        input: true,
        ..PlatformCapabilities::NONE
    };
    assert!(input_only.supports(PlatformService::Input));
    assert!(!input_only.supports(PlatformService::InputIme));
    assert!(!input_only.supports(PlatformService::Clipboard));

    let complete_host_surface = PlatformCapabilities {
        input: true,
        input_ime: true,
        clipboard: true,
        ..PlatformCapabilities::NONE
    };
    assert!(complete_host_surface.supports(PlatformService::Input));
    assert!(complete_host_surface.supports(PlatformService::InputIme));
    assert!(complete_host_surface.supports(PlatformService::Clipboard));
}
