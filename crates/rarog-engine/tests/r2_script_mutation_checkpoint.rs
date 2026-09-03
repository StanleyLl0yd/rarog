use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{
    EngineEventLoop, EngineEventLoopStep, IncrementalMode, RenderOptions, RenderSession,
};
use rarog_scheduler::{SchedulerLimits, TaskSource, WorkId};

fn element_with_id(document: &Document, expected_id: &str) -> NodeId {
    let mut stack = vec![document.root()];
    while let Some(node_id) = stack.pop() {
        let node = document.node(node_id).expect("reachable DOM node");
        if let NodeKind::Element(element) = &node.kind {
            if element.attributes.get("id").map(String::as_str) == Some(expected_id) {
                return node_id;
            }
        }
        stack.extend(node.children.iter().rev().copied());
    }
    panic!("element with id {expected_id:?} not found");
}

fn scheduler_limits() -> SchedulerLimits {
    SchedulerLimits::try_new(16, 32).unwrap()
}

#[test]
fn task_and_microtask_dom_mutations_batch_until_render_checkpoint() {
    let source = r#"<div id="target" style="width:120px;background:#112233">one</div>"#;
    let expected_source =
        r#"<div id="target" style="width:120px;background:#445566">one two three</div>"#;
    let mut session = RenderSession::new(source, RenderOptions::default()).unwrap();
    let expected = RenderSession::new(expected_source, RenderOptions::default()).unwrap();
    let target = element_with_id(session.document(), "target");
    let text = session.document().children(target).unwrap()[0];
    let initial_framebuffer = session.framebuffer().stable_hash64();
    let initial_generation = session.document().generation();

    let mut event_loop = EngineEventLoop::<&'static str, &'static str>::new(scheduler_limits()).unwrap();
    let task_id = event_loop
        .queue_task(TaskSource::DomManipulation, "script task")
        .unwrap();
    let microtask_id = event_loop.queue_microtask("promise job").unwrap();

    let task = event_loop.next_step(&mut session).unwrap().unwrap();
    assert!(matches!(task, EngineEventLoopStep::Task(_)));
    session
        .document_mut()
        .set_attribute(target, "style", "width:120px;background:#445566")
        .unwrap();
    assert_eq!(session.framebuffer().stable_hash64(), initial_framebuffer);
    event_loop.complete(WorkId::Task(task_id)).unwrap();

    let microtask = event_loop.next_step(&mut session).unwrap().unwrap();
    assert!(matches!(microtask, EngineEventLoopStep::Microtask(_)));
    session.document_mut().set_text(text, "one two three").unwrap();
    assert_eq!(session.framebuffer().stable_hash64(), initial_framebuffer);
    event_loop
        .complete(WorkId::Microtask(microtask_id))
        .unwrap();

    let checkpoint = event_loop.next_step(&mut session).unwrap().unwrap();
    let EngineEventLoopStep::RenderCheckpoint(report) = checkpoint else {
        panic!("expected retained render checkpoint");
    };
    assert_eq!(report.mode, IncrementalMode::FlowRelayout);
    assert!(report.retained_display_list);
    assert!(report.through_generation >= initial_generation + 2);
    assert_eq!(report.through_generation, session.document().generation());
    assert_eq!(
        session.framebuffer().stable_hash64(),
        expected.framebuffer().stable_hash64()
    );
    assert!(event_loop.next_step(&mut session).unwrap().is_none());
}

#[test]
fn checkpoint_without_dom_mutations_uses_the_existing_unchanged_path() {
    let mut session = RenderSession::new(
        r#"<div id="target" style="width:80px">unchanged</div>"#,
        RenderOptions::default(),
    )
    .unwrap();
    let framebuffer = session.framebuffer().stable_hash64();
    let mut event_loop = EngineEventLoop::<(), ()>::new(scheduler_limits()).unwrap();
    let task_id = event_loop
        .queue_task(TaskSource::DomManipulation, ())
        .unwrap();

    assert!(matches!(
        event_loop.next_step(&mut session).unwrap().unwrap(),
        EngineEventLoopStep::Task(_)
    ));
    event_loop.complete(WorkId::Task(task_id)).unwrap();

    let EngineEventLoopStep::RenderCheckpoint(report) =
        event_loop.next_step(&mut session).unwrap().unwrap()
    else {
        panic!("expected render checkpoint");
    };
    assert_eq!(report.mode, IncrementalMode::Unchanged);
    assert_eq!(session.framebuffer().stable_hash64(), framebuffer);
    assert!(session.damage().rects.is_empty());
}
