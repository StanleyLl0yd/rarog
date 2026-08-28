use rarog_dom::{Document, NodeId, NodeKind};
use rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};
use std::hint::black_box;
use std::time::Duration;

const FIXTURE: &str = "<style>.card { width:80px; height:20px; padding:4px; background:#112233; } #hero { border-width:2px; border-color:#000000; }</style><div id=\"hero\" class=\"card\">Rarog benchmark fixture</div>";

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(50);

    println!("Rarog R0 benchmark harness; timings are local diagnostics, not performance claims");
    println!("scenario,iterations,total_ns,average_ns");
    print_sample("full-render", iterations, benchmark_full_render(iterations));
    print_sample(
        "paint-only-update",
        iterations,
        benchmark_update(
            iterations,
            IncrementalMode::PaintOnlyReuse,
            |session, node| {
                session
                    .document_mut()
                    .set_attribute(
                        node,
                        "style",
                        "width:80px;height:20px;padding:4px;background:#445566",
                    )
                    .unwrap();
            },
        ),
    );
    print_sample(
        "subtree-relayout",
        iterations,
        benchmark_update(
            iterations,
            IncrementalMode::SubtreeRelayout,
            |session, node| {
                session
                    .document_mut()
                    .set_attribute(
                        node,
                        "style",
                        "width:96px;height:20px;padding:4px;background:#112233",
                    )
                    .unwrap();
            },
        ),
    );
    print_sample(
        "flow-relayout",
        iterations,
        benchmark_update(
            iterations,
            IncrementalMode::FlowRelayout,
            |session, node| {
                session
                    .document_mut()
                    .set_attribute(
                        node,
                        "style",
                        "width:80px;height:28px;padding:4px;background:#112233",
                    )
                    .unwrap();
            },
        ),
    );
}

fn benchmark_full_render(iterations: usize) -> Duration {
    let mut total = Duration::ZERO;
    for _ in 0..iterations {
        let output = render_html(FIXTURE, RenderOptions::default()).unwrap();
        total += output.observability.timings.total;
        black_box(output.deterministic_signature_hash());
    }
    total
}

fn benchmark_update<F>(iterations: usize, expected: IncrementalMode, mutate: F) -> Duration
where
    F: Fn(&mut RenderSession, NodeId),
{
    let mut total = Duration::ZERO;
    for _ in 0..iterations {
        let mut session = RenderSession::new(FIXTURE, RenderOptions::default()).unwrap();
        let hero = element_with_id(session.document(), "hero");
        mutate(&mut session, hero);
        let report = session.update();
        assert_eq!(report.mode, expected);
        total += report.elapsed;
        black_box(session.framebuffer().stable_hash64());
    }
    total
}

fn print_sample(name: &str, iterations: usize, total: Duration) {
    let total_ns = total.as_nanos();
    let average_ns = total_ns / iterations as u128;
    println!("{name},{iterations},{total_ns},{average_ns}");
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

    find(document, document.root(), id).expect("benchmark fixture contains requested id")
}
