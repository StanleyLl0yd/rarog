from pathlib import Path

layout = Path('crates/rarog-layout/src/lib.rs')
text = layout.read_text()

text = text.replace(
'''impl FragmentId {
    pub const fn index(self) -> usize {
        self.0
    }
}
''',
'''impl FragmentId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FragmentOrdinal(u32);

impl FragmentOrdinal {
    pub const fn index(self) -> u32 {
        self.0
    }
}
''', 1)

text = text.replace(
'''pub struct Fragment {
    pub id: FragmentId,
    pub layout_node: LayoutNodeId,
''',
'''pub struct Fragment {
    pub id: FragmentId,
    pub ordinal: FragmentOrdinal,
    pub layout_node: LayoutNodeId,
''', 1)

text = text.replace(
'''pub fn fragment_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Option<&Fragment> {
    find_fragment(&tree.root, dom_node)
}
''',
'''pub fn fragment_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Option<&Fragment> {
    find_fragment(&tree.root, dom_node)
}

pub fn fragments_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Vec<&Fragment> {
    let mut fragments = Vec::new();
    collect_fragments(&tree.root, dom_node, &mut fragments);
    fragments
}
''', 1)

text = text.replace(
'''fn find_fragment(fragment: &Fragment, dom_node: NodeId) -> Option<&Fragment> {
    if fragment.dom_node == Some(dom_node) {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| find_fragment(child, dom_node))
}
''',
'''fn find_fragment(fragment: &Fragment, dom_node: NodeId) -> Option<&Fragment> {
    if fragment.dom_node == Some(dom_node) {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| find_fragment(child, dom_node))
}

fn collect_fragments<'a>(fragment: &'a Fragment, dom_node: NodeId, output: &mut Vec<&'a Fragment>) {
    if fragment.dom_node == Some(dom_node) {
        output.push(fragment);
    }
    for child in &fragment.children {
        collect_fragments(child, dom_node, output);
    }
}
''', 1)

text = text.replace(
'''    for child in &tree.root.children[start_index..] {
        rebuilt.push(builder.layout_node(child, containing_block, &mut cursor_y));
    }
''',
'''    for child in &tree.root.children[start_index..] {
        rebuilt.extend(builder.layout_node(child, containing_block, &mut cursor_y));
    }
''', 1)

text = text.replace(
'''        if child.dom_node == Some(dom_node) {
            let mut cursor_y = child.boxes.margin_box.origin.y;
            *child = builder.layout_node(layout_node, containing_block, &mut cursor_y);
            return true;
        }
''',
'''        if child.dom_node == Some(dom_node) {
            let mut cursor_y = child.boxes.margin_box.origin.y;
            let mut replacement = builder.layout_node(layout_node, containing_block, &mut cursor_y);
            if replacement.len() != 1 {
                return false;
            }
            *child = replacement.remove(0);
            return true;
        }
''', 1)

text = text.replace(
'''        for child in &tree.root.children {
            children.push(self.layout_node(child, containing_block, &mut cursor_y));
        }
''',
'''        for child in &tree.root.children {
            children.extend(self.layout_node(child, containing_block, &mut cursor_y));
        }
''', 1)

text = text.replace(
'''            root: Fragment {
                id: self.allocate_id(),
                layout_node: tree.root.id,
''',
'''            root: Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(0),
                layout_node: tree.root.id,
''', 1)

old = '''    fn layout_node(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Fragment {
        match &node.kind {
            LayoutNodeKind::Root => unreachable!("only the layout root may have Root kind"),
            LayoutNodeKind::Text(run) => self.layout_text(node, run, containing_block, cursor_y),
            LayoutNodeKind::Box => self.layout_box(node, containing_block, cursor_y),
        }
    }

    fn layout_text(
        &mut self,
        node: &LayoutNode,
        run: &TextRun,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Fragment {
        let width = run.advance.min(containing_block.available.width.max(0.0));
        let rect = Rect::new(containing_block.origin.x, *cursor_y, width, run.line_height);
        *cursor_y += run.line_height;

        Fragment {
            id: self.allocate_id(),
            layout_node: node.id,
            dom_node: node.dom_node,
            kind: FragmentKind::Text,
            boxes: BoxModel::single(rect),
            style: node.style,
            children: Vec::new(),
        }
    }
'''
new = '''    fn layout_node(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Vec<Fragment> {
        match &node.kind {
            LayoutNodeKind::Root => unreachable!("only the layout root may have Root kind"),
            LayoutNodeKind::Text(run) => self.layout_text(node, run, containing_block, cursor_y),
            LayoutNodeKind::Box => vec![self.layout_box(node, containing_block, cursor_y)],
        }
    }

    fn layout_text(
        &mut self,
        node: &LayoutNode,
        run: &TextRun,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Vec<Fragment> {
        const BOOTSTRAP_ADVANCE: f32 = 8.0;
        let available_width = containing_block.available.width.max(0.0);
        let character_count = run.text.chars().count();
        let characters_per_fragment = if available_width >= BOOTSTRAP_ADVANCE {
            (available_width / BOOTSTRAP_ADVANCE).floor().max(1.0) as usize
        } else {
            character_count.max(1)
        };
        let fragment_count = character_count.max(1).div_ceil(characters_per_fragment);
        let mut fragments = Vec::with_capacity(fragment_count);
        let mut remaining = character_count;

        for ordinal in 0..fragment_count {
            let characters = remaining.min(characters_per_fragment);
            let width = (characters as f32 * BOOTSTRAP_ADVANCE).min(available_width);
            let rect = Rect::new(containing_block.origin.x, *cursor_y, width, run.line_height);
            *cursor_y += run.line_height;
            remaining = remaining.saturating_sub(characters);
            fragments.push(Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(ordinal as u32),
                layout_node: node.id,
                dom_node: node.dom_node,
                kind: FragmentKind::Text,
                boxes: BoxModel::single(rect),
                style: node.style,
                children: Vec::new(),
            });
        }
        fragments
    }
'''
if old not in text:
    raise SystemExit('layout text marker not found')
text = text.replace(old, new, 1)

text = text.replace(
'''        for child in &node.children {
            children.push(self.layout_node(child, child_containing_block, &mut child_y));
        }
''',
'''        for child in &node.children {
            children.extend(self.layout_node(child, child_containing_block, &mut child_y));
        }
''', 1)

text = text.replace(
'''        Fragment {
            id: self.allocate_id(),
            layout_node: node.id,
''',
'''        Fragment {
            id: self.allocate_id(),
            ordinal: FragmentOrdinal(0),
            layout_node: node.id,
''', 1)

text = text.replace(
'''        "{}fragment={}|layout={}|dom={dom}|kind={:?}|margin={}|border={}|padding={}|content={}\\n",
        " ".repeat(depth),
        fragment.id.index(),
        fragment.layout_node.index(),
''',
'''        "{}fragment={}|ordinal={}|layout={}|dom={dom}|kind={:?}|margin={}|border={}|padding={}|content={}\\n",
        " ".repeat(depth),
        fragment.id.index(),
        fragment.ordinal.index(),
        fragment.layout_node.index(),
''', 1)

module_end = text.rfind('\n}')
insert = r'''

    #[test]
    fn narrow_text_produces_multiple_fragments_for_one_layout_node() {
        let mut doc = Document::new();
        let text_node = doc
            .append_new(doc.root(), NodeKind::Text("abcdefghij".into()))
            .unwrap();
        let output = layout_document(
            &doc,
            Size {
                width: 24.0,
                height: 200.0,
            },
        );

        let layout_node = &output.tree.root.children[0];
        let fragments = fragments_for_dom(&output.fragments, text_node);
        assert_eq!(fragments.len(), 4);
        assert!(fragments.iter().all(|fragment| fragment.layout_node == layout_node.id));
        assert_eq!(
            fragments
                .iter()
                .map(|fragment| fragment.ordinal.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(fragments[0].boxes.content_box.size.width, 24.0);
        assert_eq!(fragments[3].boxes.content_box.size.width, 8.0);
    }
'''
text = text[:module_end] + insert + text[module_end:]
layout.write_text(text)

paint = Path('crates/rarog-paint/src/lib.rs')
text = paint.read_text()
text = text.replace(
'''        fragment: fragment.id.index() as u64,
''',
'''        fragment: u64::from(fragment.ordinal.index()),
''', 1)
paint.write_text(text)

backlog = Path('docs/R0-BACKLOG.md')
text = backlog.read_text().replace(
    '- [ ] fragmentation cases that produce multiple fragments per layout node',
    '- [x] bootstrap text fragmentation can produce multiple fragments per layout node with stable ordinals',
    1,
)
backlog.write_text(text)

architecture = Path('docs/ARCHITECTURE.md')
text = architecture.read_text()
needle = 'Retained display-list replacement operates on exact contiguous command ranges'
pos = text.find(needle)
if pos < 0:
    raise SystemExit('retained architecture marker not found')
paragraph_end = text.find('\n\n', pos)
addition = '\n\nFragment identity is explicitly one-to-many with layout identity. A layout node may emit multiple fragments, each carrying a stable ordinal within that source node. The R0 proof case uses bootstrap fixed-advance text fragmentation in narrow containing blocks; it is an architectural multiplicity test, not a standards line-breaking implementation. Display-item identity uses the fragment ordinal rather than the ephemeral FragmentId so multiple fragments remain distinct without coupling retained paint to snapshot allocation order.'
text = text[:paragraph_end] + addition + text[paragraph_end:]
architecture.write_text(text)
