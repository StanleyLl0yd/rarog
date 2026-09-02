from pathlib import Path

layout = Path("crates/rarog-layout/src/lib.rs")
s = layout.read_text()
old = '''pub fn relayout_fragment_flow(
    tree: &LayoutTree,
    fragments: &mut FragmentTree,
    dirty_nodes: &[NodeId],
) -> bool {
    if dirty_nodes.is_empty() || tree.root.children.len() != fragments.root.children.len() {
        return false;
    }

    let Some(start_index) = tree
        .root
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| {
            dirty_nodes
                .iter()
                .any(|dirty| layout_node_contains(child, *dirty))
        })
        .map(|(index, _)| index)
        .min()
    else {
        return false;
    };
'''
new = '''pub fn fragment_flow_start_index(
    tree: &LayoutTree,
    fragments: &FragmentTree,
    dirty_nodes: &[NodeId],
) -> Option<usize> {
    if dirty_nodes.is_empty() || tree.root.children.len() != fragments.root.children.len() {
        return None;
    }

    tree.root
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| {
            dirty_nodes
                .iter()
                .any(|dirty| layout_node_contains(child, *dirty))
        })
        .map(|(index, _)| index)
        .min()
}

pub fn relayout_fragment_flow(
    tree: &LayoutTree,
    fragments: &mut FragmentTree,
    dirty_nodes: &[NodeId],
) -> bool {
    let Some(start_index) = fragment_flow_start_index(tree, fragments, dirty_nodes) else {
        return false;
    };
'''
if "pub fn fragment_flow_start_index(" not in s:
    if old not in s:
        raise SystemExit("layout flow marker missing")
    s = s.replace(old, new, 1)
    layout.write_text(s)

paint = Path("crates/rarog-paint/src/lib.rs")
s = paint.read_text()
marker = '''pub fn replace_display_items_for_fragment(
    list: &mut DisplayList,
    previous: &Fragment,
    current: &Fragment,
) -> bool {
'''
addition = '''pub fn build_display_list_for_fragments(fragments: &[Fragment]) -> DisplayList {
    let mut list = DisplayList::default();
    for fragment in fragments {
        collect(fragment, &mut list);
    }
    list
}

pub fn replace_display_items_for_fragments(
    list: &mut DisplayList,
    previous: &[Fragment],
    current: &[Fragment],
) -> bool {
    let previous_items = build_display_list_for_fragments(previous);
    let current_items = build_display_list_for_fragments(current);
    replace_display_items(list, &previous_items, &current_items)
}

'''
if "pub fn replace_display_items_for_fragments(" not in s:
    if marker not in s:
        raise SystemExit("paint insertion marker missing")
    s = s.replace(marker, addition + marker, 1)
    paint.write_text(s)

engine = Path("crates/rarog-engine/src/lib.rs")
s = engine.read_text()
s = s.replace(
    "    Fragment, LayoutNode, LayoutOutput, build_layout_tree, fragment_for_dom, fragments_for_dom,\n    layout_document_with_styles, refresh_text_node, relayout_fragment_flow,\n",
    "    Fragment, LayoutNode, LayoutOutput, build_layout_tree, fragment_flow_start_index,\n    fragment_for_dom, fragments_for_dom, layout_document_with_styles, refresh_text_node,\n    relayout_fragment_flow,\n",
    1,
)
s = s.replace(
    "    DamageRegion, DisplayList, Framebuffer, FramebufferError, build_display_list,\n    replace_display_items_for_fragment,\n",
    "    DamageRegion, DisplayList, Framebuffer, FramebufferError, build_display_list,\n    replace_display_items_for_fragment, replace_display_items_for_fragments,\n",
    1,
)

if "pub retained_display_list: bool," not in s:
    s = s.replace(
        "    pub patched_nodes: usize,\n    pub elapsed: Duration,\n",
        "    pub patched_nodes: usize,\n    pub retained_display_list: bool,\n    pub elapsed: Duration,\n",
        1,
    )
    s = s.replace(
        "                patched_nodes: 0,\n                elapsed: update_started.elapsed(),\n",
        "                patched_nodes: 0,\n                retained_display_list: true,\n                elapsed: update_started.elapsed(),\n",
        1,
    )

if "let retained_display_list;" not in s:
    s = s.replace(
        "        let mode;\n        let patched_nodes;\n",
        "        let mode;\n        let patched_nodes;\n        let retained_display_list;\n",
        1,
    )
    s = s.replace(
        "            mode = IncrementalMode::FullRebuild;\n            patched_nodes = 0;\n",
        "            mode = IncrementalMode::FullRebuild;\n            patched_nodes = 0;\n            retained_display_list = false;\n",
        1,
    )
    s = s.replace(
        "            mode = IncrementalMode::Unchanged;\n            patched_nodes = 0;\n",
        "            mode = IncrementalMode::Unchanged;\n            patched_nodes = 0;\n            retained_display_list = true;\n",
        1,
    )
    s = s.replace(
        "                mode = IncrementalMode::SubtreeRelayout;\n            } else {\n                self.layout.fragments = relayout_tree(&self.layout.tree, self.options.viewport);\n                self.display_list = build_display_list(&self.layout.fragments);\n                mode = IncrementalMode::GeometryRelayout;\n",
        "                mode = IncrementalMode::SubtreeRelayout;\n                retained_display_list = retained_display;\n            } else {\n                self.layout.fragments = relayout_tree(&self.layout.tree, self.options.viewport);\n                self.display_list = build_display_list(&self.layout.fragments);\n                mode = IncrementalMode::GeometryRelayout;\n                retained_display_list = false;\n",
        1,
    )

    old_flow = '''            self.styles = new_styles;
            let flow_nodes = flow_relayout_nodes.into_iter().collect::<Vec<_>>();
            if relayout_fragment_flow(&self.layout.tree, &mut self.layout.fragments, &flow_nodes) {
                mode = IncrementalMode::FlowRelayout;
            } else {
                self.layout.fragments = relayout_tree(&self.layout.tree, self.options.viewport);
                mode = IncrementalMode::GeometryRelayout;
            }
            self.display_list = build_display_list(&self.layout.fragments);
'''
    new_flow = '''            self.styles = new_styles;
            let flow_nodes = flow_relayout_nodes.into_iter().collect::<Vec<_>>();
            let flow_start =
                fragment_flow_start_index(&self.layout.tree, &self.layout.fragments, &flow_nodes);
            let previous_flow_fragments = flow_start
                .map(|start| self.layout.fragments.root.children[start..].to_vec());
            if relayout_fragment_flow(&self.layout.tree, &mut self.layout.fragments, &flow_nodes) {
                mode = IncrementalMode::FlowRelayout;
                let retained_display = match (flow_start, previous_flow_fragments.as_deref()) {
                    (Some(start), Some(previous)) => replace_display_items_for_fragments(
                        &mut self.display_list,
                        previous,
                        &self.layout.fragments.root.children[start..],
                    ),
                    _ => false,
                };
                if !retained_display {
                    self.display_list = build_display_list(&self.layout.fragments);
                }
                retained_display_list = retained_display;
            } else {
                self.layout.fragments = relayout_tree(&self.layout.tree, self.options.viewport);
                self.display_list = build_display_list(&self.layout.fragments);
                mode = IncrementalMode::GeometryRelayout;
                retained_display_list = false;
            }
'''
    if old_flow not in s:
        raise SystemExit("engine flow marker missing")
    s = s.replace(old_flow, new_flow, 1)

    s = s.replace(
        "            mode = IncrementalMode::PaintOnlyReuse;\n        }\n\n        self.dirty.clear();\n",
        "            mode = IncrementalMode::PaintOnlyReuse;\n            retained_display_list = retained_display;\n        }\n\n        self.dirty.clear();\n",
        1,
    )
    s = s.replace(
        "            patched_nodes,\n            elapsed: update_started.elapsed(),\n",
        "            patched_nodes,\n            retained_display_list,\n            elapsed: update_started.elapsed(),\n",
        1,
    )

# Every engine unit test that already expects FlowRelayout should also prove retained paint.
needle = "        assert_eq!(report.mode, IncrementalMode::FlowRelayout);\n"
replacement = needle + "        assert!(report.retained_display_list);\n"
if "assert!(report.retained_display_list);" not in s:
    s = s.replace(needle, replacement)

engine.write_text(s)
