from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker: {label}")
    return text.replace(old, new, 1)


def update_dom() -> None:
    path = Path("crates/rarog-dom/src/lib.rs")
    text = path.read_text()
    text = replace_once(
        text,
        '''    pub fn generation(&self) -> u64 {\n        self.generation\n    }\n\n    pub fn contains(&self, id: NodeId) -> bool {\n''',
        '''    pub fn generation(&self) -> u64 {\n        self.generation\n    }\n\n    pub fn node_count(&self) -> usize {\n        self.nodes.len()\n    }\n\n    pub fn contains(&self, id: NodeId) -> bool {\n''',
        "DOM node count",
    )
    path.write_text(text)


def update_layout() -> None:
    path = Path("crates/rarog-layout/src/lib.rs")
    text = path.read_text()
    text = replace_once(
        text,
        '''    pub fn style_snapshot(&self) -> String {\n        let mut output = String::new();\n        snapshot_style_node(&self.root, &mut output);\n        output\n    }\n}\n''',
        '''    pub fn style_snapshot(&self) -> String {\n        let mut output = String::new();\n        snapshot_style_node(&self.root, &mut output);\n        output\n    }\n\n    pub fn node_count(&self) -> usize {\n        count_layout_nodes(&self.root)\n    }\n}\n\nfn count_layout_nodes(node: &LayoutNode) -> usize {\n    1 + node.children.iter().map(count_layout_nodes).sum::<usize>()\n}\n''',
        "layout node count",
    )
    text = replace_once(
        text,
        '''impl FragmentTree {\n    pub fn snapshot(&self) -> String {\n        let mut output = String::new();\n        snapshot_fragment(&self.root, 0, &mut output);\n        output\n    }\n}\n''',
        '''impl FragmentTree {\n    pub fn snapshot(&self) -> String {\n        let mut output = String::new();\n        snapshot_fragment(&self.root, 0, &mut output);\n        output\n    }\n\n    pub fn fragment_count(&self) -> usize {\n        count_fragments(&self.root)\n    }\n}\n\nfn count_fragments(fragment: &Fragment) -> usize {\n    1 + fragment.children.iter().map(count_fragments).sum::<usize>()\n}\n''',
        "fragment count",
    )
    text = replace_once(
        text,
        '''pub fn layout_document_with_styles(\n    doc: &Document,\n    styles: &StyleSet,\n    viewport: Size,\n) -> LayoutOutput {\n    let mut tree_builder = LayoutTreeBuilder::new(styles);\n    let root = tree_builder\n        .build_node(doc, doc.root())\n        .expect("document root always creates a layout root");\n    let tree = LayoutTree { root };\n\n    let fragments = relayout_tree(&tree, viewport);\n\n    LayoutOutput { tree, fragments }\n}\n''',
        '''pub fn build_layout_tree(doc: &Document, styles: &StyleSet) -> LayoutTree {\n    let mut tree_builder = LayoutTreeBuilder::new(styles);\n    let root = tree_builder\n        .build_node(doc, doc.root())\n        .expect("document root always creates a layout root");\n    LayoutTree { root }\n}\n\npub fn layout_document_with_styles(\n    doc: &Document,\n    styles: &StyleSet,\n    viewport: Size,\n) -> LayoutOutput {\n    let tree = build_layout_tree(doc, styles);\n    let fragments = relayout_tree(&tree, viewport);\n    LayoutOutput { tree, fragments }\n}\n''',
        "layout stage split",
    )
    path.write_text(text)


def update_engine() -> None:
    path = Path("crates/rarog-engine/src/lib.rs")
    text = path.read_text()
    text = replace_once(
        text,
        '''use rarog_layout::{\n    Fragment, LayoutNode, LayoutOutput, fragment_for_dom, layout_document_with_styles,\n    relayout_fragment_flow, relayout_fragment_subtree, relayout_tree,\n};\n''',
        '''use rarog_layout::{\n    Fragment, LayoutNode, LayoutOutput, build_layout_tree, fragment_for_dom,\n    layout_document_with_styles, relayout_fragment_flow, relayout_fragment_subtree, relayout_tree,\n};\n''',
        "engine layout imports",
    )
    text = replace_once(
        text,
        '''use std::collections::{BTreeMap, BTreeSet};\n''',
        '''use std::collections::{BTreeMap, BTreeSet};\nuse std::time::{Duration, Instant};\n''',
        "engine time import",
    )
    text = replace_once(
        text,
        '''impl Default for RenderOptions {\n    fn default() -> Self {\n        Self {\n            viewport: Size {\n                width: 1024.0,\n                height: 768.0,\n            },\n            background: Color::WHITE,\n        }\n    }\n}\n\npub struct RenderOutput {\n''',
        '''impl Default for RenderOptions {\n    fn default() -> Self {\n        Self {\n            viewport: Size {\n                width: 1024.0,\n                height: 768.0,\n            },\n            background: Color::WHITE,\n        }\n    }\n}\n\n#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\npub struct RenderTimings {\n    pub parse: Duration,\n    pub style: Duration,\n    pub layout_tree: Duration,\n    pub fragment: Duration,\n    pub paint_list: Duration,\n    pub raster: Duration,\n    pub total: Duration,\n}\n\n#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\npub struct RenderCounters {\n    pub dom_nodes: usize,\n    pub layout_nodes: usize,\n    pub fragments: usize,\n    pub display_commands: usize,\n    pub damage_rects: usize,\n}\n\n#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\npub struct RenderObservability {\n    pub timings: RenderTimings,\n    pub counters: RenderCounters,\n}\n\npub struct RenderOutput {\n''',
        "render observability structs",
    )
    text = replace_once(
        text,
        '''    pub damage: DamageRegion,\n    pub framebuffer: Framebuffer,\n}\n''',
        '''    pub damage: DamageRegion,\n    pub framebuffer: Framebuffer,\n    pub observability: RenderObservability,\n}\n''',
        "render output observability",
    )
    text = replace_once(
        text,
        '''pub struct IncrementalReport {\n    pub mode: IncrementalMode,\n    pub from_generation: u64,\n    pub through_generation: u64,\n    pub dirty_nodes: usize,\n    pub patched_nodes: usize,\n}\n''',
        '''pub struct IncrementalReport {\n    pub mode: IncrementalMode,\n    pub from_generation: u64,\n    pub through_generation: u64,\n    pub dirty_nodes: usize,\n    pub patched_nodes: usize,\n    pub elapsed: Duration,\n}\n''',
        "incremental elapsed",
    )
    text = replace_once(
        text,
        '''    pub fn update(&mut self) -> IncrementalReport {\n        let from_generation = self.dirty.through_generation();\n''',
        '''    pub fn update(&mut self) -> IncrementalReport {\n        let update_started = Instant::now();\n        let from_generation = self.dirty.through_generation();\n''',
        "update timer start",
    )
    text = replace_once(
        text,
        '''                dirty_nodes,\n                patched_nodes: 0,\n            };\n''',
        '''                dirty_nodes,\n                patched_nodes: 0,\n                elapsed: update_started.elapsed(),\n            };\n''',
        "unchanged incremental elapsed",
    )
    text = replace_once(
        text,
        '''            dirty_nodes,\n            patched_nodes,\n        }\n    }\n''',
        '''            dirty_nodes,\n            patched_nodes,\n            elapsed: update_started.elapsed(),\n        }\n    }\n''',
        "final incremental elapsed",
    )
    text = replace_once(
        text,
        '''pub fn render_html_against(\n    source: &str,\n    options: RenderOptions,\n    previous_display_list: Option<&DisplayList>,\n) -> Result<RenderOutput, RenderError> {\n    let document = rarog_html::parse(source);\n    let styles = StyleSet::for_document(&document);\n    let layout = layout_document_with_styles(&document, &styles, options.viewport);\n    let display_list = build_display_list(&layout.fragments);\n    let damage = DamageRegion::between(previous_display_list, &display_list);\n    let mut framebuffer = Framebuffer::try_new(options.viewport, options.background)?;\n    framebuffer.rasterize(&display_list);\n\n    Ok(RenderOutput {\n        document,\n        styles,\n        layout,\n        display_list,\n        damage,\n        framebuffer,\n    })\n}\n''',
        '''pub fn render_html_against(\n    source: &str,\n    options: RenderOptions,\n    previous_display_list: Option<&DisplayList>,\n) -> Result<RenderOutput, RenderError> {\n    let total_started = Instant::now();\n\n    let stage_started = Instant::now();\n    let document = rarog_html::parse(source);\n    let parse = stage_started.elapsed();\n\n    let stage_started = Instant::now();\n    let styles = StyleSet::for_document(&document);\n    let style = stage_started.elapsed();\n\n    let stage_started = Instant::now();\n    let tree = build_layout_tree(&document, &styles);\n    let layout_tree = stage_started.elapsed();\n\n    let stage_started = Instant::now();\n    let fragments = relayout_tree(&tree, options.viewport);\n    let fragment = stage_started.elapsed();\n    let layout = LayoutOutput { tree, fragments };\n\n    let stage_started = Instant::now();\n    let display_list = build_display_list(&layout.fragments);\n    let damage = DamageRegion::between(previous_display_list, &display_list);\n    let paint_list = stage_started.elapsed();\n\n    let stage_started = Instant::now();\n    let mut framebuffer = Framebuffer::try_new(options.viewport, options.background)?;\n    framebuffer.rasterize(&display_list);\n    let raster = stage_started.elapsed();\n\n    let observability = RenderObservability {\n        timings: RenderTimings {\n            parse,\n            style,\n            layout_tree,\n            fragment,\n            paint_list,\n            raster,\n            total: total_started.elapsed(),\n        },\n        counters: RenderCounters {\n            dom_nodes: document.node_count(),\n            layout_nodes: layout.tree.node_count(),\n            fragments: layout.fragments.fragment_count(),\n            display_commands: display_list.commands.len(),\n            damage_rects: damage.rects.len(),\n        },\n    };\n\n    Ok(RenderOutput {\n        document,\n        styles,\n        layout,\n        display_list,\n        damage,\n        framebuffer,\n        observability,\n    })\n}\n''',
        "instrumented full render",
    )
    test_anchor = '''    #[test]\n    fn bootstrap_pipeline_produces_commands_and_fragments() {\n'''
    observability_test = '''    #[test]\n    fn full_render_exposes_stage_observability_without_affecting_identity() {\n        let first = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());\n        let second = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());\n        let counters = first.observability.counters;\n\n        assert_eq!(counters.dom_nodes, first.document.node_count());\n        assert_eq!(counters.layout_nodes, first.layout.tree.node_count());\n        assert_eq!(counters.fragments, first.layout.fragments.fragment_count());\n        assert_eq!(counters.display_commands, first.display_list.commands.len());\n        assert_eq!(counters.damage_rects, first.damage.rects.len());\n        assert!(first.observability.timings.total >= first.observability.timings.raster);\n        assert_eq!(\n            first.deterministic_signature_hash(),\n            second.deterministic_signature_hash()\n        );\n    }\n\n    #[test]\n    fn incremental_report_exposes_elapsed_time_and_path_counts() {\n        let mut session = session(DETERMINISTIC_FIXTURE, deterministic_options());\n        let hero = element_with_id(session.document(), "hero");\n        session\n            .document_mut()\n            .set_attribute(hero, "style", "background:#445566")\n            .unwrap();\n\n        let report = session.update();\n        assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);\n        assert_eq!(report.dirty_nodes, 1);\n        assert_eq!(report.patched_nodes, 1);\n        let _elapsed = report.elapsed;\n    }\n\n'''
    text = replace_once(text, test_anchor, observability_test + test_anchor, "observability tests")
    path.write_text(text)


def add_benchmark_example() -> None:
    path = Path("crates/rarog-engine/examples/r0_bench.rs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        '''use rarog_dom::{Document, NodeId, NodeKind};\nuse rarog_engine::{IncrementalMode, RenderOptions, RenderSession, render_html};\nuse std::hint::black_box;\nuse std::time::Duration;\n\nconst FIXTURE: &str = "<style>.card { width:80px; height:20px; padding:4px; background:#112233; } #hero { border-width:2px; border-color:#000000; }</style><div id=\\\"hero\\\" class=\\\"card\\\">Rarog benchmark fixture</div>";\n\nfn main() {\n    let iterations = std::env::args()\n        .nth(1)\n        .and_then(|value| value.parse::<usize>().ok())\n        .filter(|value| *value > 0)\n        .unwrap_or(50);\n\n    println!("Rarog R0 benchmark harness; timings are local diagnostics, not performance claims");\n    println!("scenario,iterations,total_ns,average_ns");\n    print_sample("full-render", iterations, benchmark_full_render(iterations));\n    print_sample(\n        "paint-only-update",\n        iterations,\n        benchmark_update(iterations, IncrementalMode::PaintOnlyReuse, |session, node| {\n            session\n                .document_mut()\n                .set_attribute(\n                    node,\n                    "style",\n                    "width:80px;height:20px;padding:4px;background:#445566",\n                )\n                .unwrap();\n        }),\n    );\n    print_sample(\n        "subtree-relayout",\n        iterations,\n        benchmark_update(iterations, IncrementalMode::SubtreeRelayout, |session, node| {\n            session\n                .document_mut()\n                .set_attribute(\n                    node,\n                    "style",\n                    "width:96px;height:20px;padding:4px;background:#112233",\n                )\n                .unwrap();\n        }),\n    );\n    print_sample(\n        "flow-relayout",\n        iterations,\n        benchmark_update(iterations, IncrementalMode::FlowRelayout, |session, node| {\n            session\n                .document_mut()\n                .set_attribute(\n                    node,\n                    "style",\n                    "width:80px;height:28px;padding:4px;background:#112233",\n                )\n                .unwrap();\n        }),\n    );\n}\n\nfn benchmark_full_render(iterations: usize) -> Duration {\n    let mut total = Duration::ZERO;\n    for _ in 0..iterations {\n        let output = render_html(FIXTURE, RenderOptions::default()).unwrap();\n        total += output.observability.timings.total;\n        black_box(output.deterministic_signature_hash());\n    }\n    total\n}\n\nfn benchmark_update<F>(iterations: usize, expected: IncrementalMode, mutate: F) -> Duration\nwhere\n    F: Fn(&mut RenderSession, NodeId),\n{\n    let mut total = Duration::ZERO;\n    for _ in 0..iterations {\n        let mut session = RenderSession::new(FIXTURE, RenderOptions::default()).unwrap();\n        let hero = element_with_id(session.document(), "hero");\n        mutate(&mut session, hero);\n        let report = session.update();\n        assert_eq!(report.mode, expected);\n        total += report.elapsed;\n        black_box(session.framebuffer().stable_hash64());\n    }\n    total\n}\n\nfn print_sample(name: &str, iterations: usize, total: Duration) {\n    let total_ns = total.as_nanos();\n    let average_ns = total_ns / iterations as u128;\n    println!("{name},{iterations},{total_ns},{average_ns}");\n}\n\nfn element_with_id(document: &Document, id: &str) -> NodeId {\n    fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {\n        if let NodeKind::Element(element) = &document.node(node).kind\n            && element.attributes.get("id").map(String::as_str) == Some(id)\n        {\n            return Some(node);\n        }\n        document\n            .children(node)\n            .iter()\n            .find_map(|child| find(document, *child, id))\n    }\n\n    find(document, document.root(), id).expect("benchmark fixture contains requested id")\n}\n'''
    )


def update_docs() -> None:
    backlog = Path("docs/R0-BACKLOG.md")
    text = backlog.read_text()
    text = text.replace(
        "- [ ] benchmark harness with no performance claims yet",
        "- [x] benchmark harness with no performance claims yet",
    )
    backlog.write_text(text)

    architecture = Path("docs/ARCHITECTURE.md")
    text = architecture.read_text()
    anchor = "## First incremental reuse experiment\n"
    section = '''## R0 observability and benchmark harness\n\nFull bootstrap renders expose `RenderObservability` without feeding timing data into deterministic render identity. `RenderTimings` records wall-clock durations for decoded HTML parsing, style-source construction, Layout Tree construction, Fragment Tree construction, display-list/damage construction, rasterization, and the enclosing render. `RenderCounters` records DOM nodes, layout nodes, fragments, display commands, and damage rectangles. Layout Tree construction currently includes per-element computed-style resolution because R0 resolves styles while deriving layout nodes.\n\nStateful updates expose elapsed wall-clock time alongside the existing `IncrementalMode`, dirty-node count and patched-node count. These values are diagnostics only: CI does not enforce thresholds and the project makes no cross-machine performance claims from them. Allocator-backed peak/persistent byte accounting is deliberately deferred rather than publishing misleading estimates.\n\n`cargo run -p rarog-engine --example r0_bench --release -- <iterations>` runs fixed full-render, paint-only, subtree-relayout and flow-relayout scenarios. Setup for each incremental sample is excluded from the reported update duration through the engine's own timing boundary. The harness is intended to detect gross regressions during development and to provide a stable place for later benchmark methodology, not to publish competitive numbers. See ADR-0028.\n\n'''
    if "## R0 observability and benchmark harness\n" not in text:
        text = replace_once(text, anchor, section + anchor, "architecture observability anchor")
    architecture.write_text(text)

    adr = Path("docs/adr/ADR-0028-r0-observability-and-benchmark-harness.md")
    adr.write_text(
        '''# ADR-0028: R0 observability and benchmark harness\n\n## Status\n\nAccepted.\n\n## Context\n\nR0 already has deterministic correctness gates and several incremental rendering paths, but it had no stable timing/counter boundary and no reproducible harness for exercising those paths. Ad-hoc wall-clock measurements are easy to misinterpret and must not become public performance claims.\n\n## Decision\n\nFull renders expose backend-neutral `RenderObservability` containing wall-clock stage timings and structural counters. Timing data is intentionally excluded from deterministic hashes and snapshots. The layout stage is split at a public `build_layout_tree` boundary so Layout Tree and Fragment Tree construction can be observed separately without changing their identities.\n\n`IncrementalReport` carries total update elapsed time in addition to its existing mode, generation and dirty/patched-node counts. R0 does not attempt allocator instrumentation or fabricated memory-byte estimates; real peak and persistent memory accounting will require a later tracing/allocator boundary.\n\nA dependency-free `rarog-engine` example provides fixed full-render, paint-only, subtree-relayout and flow-relayout scenarios. It accepts an iteration count and prints simple CSV-compatible samples. CI compiles the harness but does not enforce latency thresholds.\n\n## Consequences\n\n- render-stage timings can be inspected without changing deterministic render identity;\n- incremental path timing is directly associated with the path report that produced it;\n- structural counters give context to local timing samples;\n- benchmark inputs and scenario semantics live in the repository and can evolve under review;\n- local measurements remain diagnostic and must not be described as cross-browser or cross-machine performance claims;\n- allocator-backed memory observability remains explicit future work rather than an R0 estimate.\n'''
    )


update_dom()
update_layout()
update_engine()
add_benchmark_example()
update_docs()
