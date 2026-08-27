use rarog_css::{ComputedStyle, StyleSet, computed_style};
use rarog_dom::{Document, NodeId, NodeKind};
use rarog_types::{Point, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutNodeId(usize);

impl LayoutNodeId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FragmentId(usize);

impl FragmentId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntrinsicSizes {
    pub min_content: f32,
    pub max_content: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub advance: f32,
    pub line_height: f32,
}

impl TextRun {
    pub fn new(text: String) -> Self {
        let advance = text.chars().count() as f32 * 8.0;
        Self {
            text,
            advance,
            line_height: 18.0,
        }
    }

    pub fn intrinsic_sizes(&self) -> IntrinsicSizes {
        let longest_word = self
            .text
            .split_whitespace()
            .map(|word| word.chars().count())
            .max()
            .unwrap_or(0) as f32
            * 8.0;
        IntrinsicSizes {
            min_content: longest_word,
            max_content: self.advance,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainingBlock {
    pub origin: Point,
    pub available: Size,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutNodeKind {
    Root,
    Box,
    Text(TextRun),
}

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub id: LayoutNodeId,
    pub dom_node: Option<NodeId>,
    pub kind: LayoutNodeKind,
    pub style: ComputedStyle,
    pub intrinsic: IntrinsicSizes,
    pub children: Vec<LayoutNode>,
}

#[derive(Clone, Debug)]
pub struct LayoutTree {
    pub root: LayoutNode,
}

impl LayoutTree {
    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        snapshot_layout_node(&self.root, 0, &mut output);
        output
    }

    pub fn style_snapshot(&self) -> String {
        let mut output = String::new();
        snapshot_style_node(&self.root, &mut output);
        output
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxModel {
    pub margin_box: Rect,
    pub border_box: Rect,
    pub padding_box: Rect,
    pub content_box: Rect,
}

impl BoxModel {
    pub const fn single(rect: Rect) -> Self {
        Self {
            margin_box: rect,
            border_box: rect,
            padding_box: rect,
            content_box: rect,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentKind {
    Root,
    Box,
    Text,
}

#[derive(Clone, Debug)]
pub struct Fragment {
    pub id: FragmentId,
    pub layout_node: LayoutNodeId,
    pub dom_node: Option<NodeId>,
    pub kind: FragmentKind,
    pub boxes: BoxModel,
    pub style: ComputedStyle,
    pub children: Vec<Fragment>,
}

#[derive(Clone, Debug)]
pub struct FragmentTree {
    pub root: Fragment,
}

impl FragmentTree {
    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        snapshot_fragment(&self.root, 0, &mut output);
        output
    }
}

#[derive(Clone, Debug)]
pub struct LayoutOutput {
    pub tree: LayoutTree,
    pub fragments: FragmentTree,
}

pub fn layout_document(doc: &Document, viewport: Size) -> LayoutOutput {
    let styles = StyleSet::for_document(doc);
    layout_document_with_styles(doc, &styles, viewport)
}

pub fn layout_document_with_styles(
    doc: &Document,
    styles: &StyleSet,
    viewport: Size,
) -> LayoutOutput {
    let mut tree_builder = LayoutTreeBuilder::new(styles);
    let root = tree_builder
        .build_node(doc, doc.root())
        .expect("document root always creates a layout root");
    let tree = LayoutTree { root };

    let fragments = relayout_tree(&tree, viewport);

    LayoutOutput { tree, fragments }
}

pub fn relayout_tree(tree: &LayoutTree, viewport: Size) -> FragmentTree {
    let mut fragment_builder = FragmentBuilder::default();
    fragment_builder.build(tree, viewport)
}

pub fn fragment_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Option<&Fragment> {
    find_fragment(&tree.root, dom_node)
}

pub fn relayout_fragment_subtree(
    tree: &LayoutTree,
    fragments: &mut FragmentTree,
    dom_node: NodeId,
) -> bool {
    let Some(layout_node) = find_layout_node(&tree.root, dom_node) else {
        return false;
    };
    let next_id = max_fragment_id(&fragments.root).saturating_add(1);
    let mut builder = FragmentBuilder { next_id };
    relayout_fragment_child(&mut fragments.root, layout_node, dom_node, &mut builder)
}

pub fn relayout_fragment_flow(
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

    let containing_block = ContainingBlock {
        origin: fragments.root.boxes.content_box.origin,
        available: fragments.root.boxes.content_box.size,
    };
    let mut cursor_y = if start_index == 0 {
        containing_block.origin.y
    } else {
        let previous = &fragments.root.children[start_index - 1];
        previous.boxes.margin_box.origin.y + previous.boxes.margin_box.size.height
    };
    let next_id = max_fragment_id(&fragments.root).saturating_add(1);
    let mut builder = FragmentBuilder { next_id };
    let mut rebuilt = Vec::with_capacity(tree.root.children.len() - start_index);

    for child in &tree.root.children[start_index..] {
        rebuilt.push(builder.layout_node(child, containing_block, &mut cursor_y));
    }

    fragments.root.children.truncate(start_index);
    fragments.root.children.extend(rebuilt);
    true
}

fn layout_node_contains(node: &LayoutNode, dom_node: NodeId) -> bool {
    node.dom_node == Some(dom_node)
        || node
            .children
            .iter()
            .any(|child| layout_node_contains(child, dom_node))
}

pub fn relayout_fragment_flow(
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

    let containing_block = ContainingBlock {
        origin: fragments.root.boxes.content_box.origin,
        available: fragments.root.boxes.content_box.size,
    };
    let mut cursor_y = if start_index == 0 {
        containing_block.origin.y
    } else {
        let previous = &fragments.root.children[start_index - 1];
        previous.boxes.margin_box.origin.y + previous.boxes.margin_box.size.height
    };
    let next_id = max_fragment_id(&fragments.root).saturating_add(1);
    let mut builder = FragmentBuilder { next_id };
    let mut rebuilt = Vec::with_capacity(tree.root.children.len() - start_index);

    for child in &tree.root.children[start_index..] {
        rebuilt.push(builder.layout_node(child, containing_block, &mut cursor_y));
    }

    fragments.root.children.truncate(start_index);
    fragments.root.children.extend(rebuilt);
    true
}

fn layout_node_contains(node: &LayoutNode, dom_node: NodeId) -> bool {
    node.dom_node == Some(dom_node)
        || node
            .children
            .iter()
            .any(|child| layout_node_contains(child, dom_node))
}

fn find_layout_node(node: &LayoutNode, dom_node: NodeId) -> Option<&LayoutNode> {
    if node.dom_node == Some(dom_node) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_layout_node(child, dom_node))
}

fn find_fragment(fragment: &Fragment, dom_node: NodeId) -> Option<&Fragment> {
    if fragment.dom_node == Some(dom_node) {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| find_fragment(child, dom_node))
}

fn max_fragment_id(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .map(max_fragment_id)
        .fold(fragment.id.index(), usize::max)
}

fn relayout_fragment_child(
    parent: &mut Fragment,
    layout_node: &LayoutNode,
    dom_node: NodeId,
    builder: &mut FragmentBuilder,
) -> bool {
    let containing_block = ContainingBlock {
        origin: parent.boxes.content_box.origin,
        available: parent.boxes.content_box.size,
    };

    for child in &mut parent.children {
        if child.dom_node == Some(dom_node) {
            let mut cursor_y = child.boxes.margin_box.origin.y;
            *child = builder.layout_node(layout_node, containing_block, &mut cursor_y);
            return true;
        }
        if relayout_fragment_child(child, layout_node, dom_node, builder) {
            return true;
        }
    }
    false
}

struct LayoutTreeBuilder<'a> {
    next_id: usize,
    styles: &'a StyleSet,
}

impl<'a> LayoutTreeBuilder<'a> {
    fn new(styles: &'a StyleSet) -> Self {
        Self { next_id: 0, styles }
    }

    fn build_node(&mut self, doc: &Document, node: NodeId) -> Option<LayoutNode> {
        let (kind, style) = match &doc.node(node).kind {
            NodeKind::Document => (LayoutNodeKind::Root, ComputedStyle::default()),
            NodeKind::Text(text) => (
                LayoutNodeKind::Text(TextRun::new(text.clone())),
                ComputedStyle::default(),
            ),
            NodeKind::Element(_) => {
                let style = computed_style(doc, node, self.styles);
                if style.display_none {
                    return None;
                }
                (LayoutNodeKind::Box, style)
            }
        };

        let id = self.allocate_id();
        let mut children = Vec::new();
        for child in doc.children(node) {
            if let Some(layout_child) = self.build_node(doc, *child) {
                children.push(layout_child);
            }
        }

        let intrinsic = intrinsic_sizes_for_node(&kind, style, &children);

        Some(LayoutNode {
            id,
            dom_node: Some(node),
            kind,
            style,
            intrinsic,
            children,
        })
    }

    fn allocate_id(&mut self) -> LayoutNodeId {
        let id = LayoutNodeId(self.next_id);
        self.next_id += 1;
        id
    }
}

fn intrinsic_sizes_for_node(
    kind: &LayoutNodeKind,
    style: ComputedStyle,
    children: &[LayoutNode],
) -> IntrinsicSizes {
    match kind {
        LayoutNodeKind::Text(run) => run.intrinsic_sizes(),
        LayoutNodeKind::Root => IntrinsicSizes {
            min_content: children
                .iter()
                .map(|child| child.intrinsic.min_content)
                .fold(0.0, f32::max),
            max_content: children
                .iter()
                .map(|child| child.intrinsic.max_content)
                .fold(0.0, f32::max),
        },
        LayoutNodeKind::Box => {
            let horizontal_edges = style.padding.horizontal() + style.border_width.horizontal();
            let child_min = children
                .iter()
                .map(|child| child.intrinsic.min_content)
                .fold(0.0, f32::max);
            let child_max = children
                .iter()
                .map(|child| child.intrinsic.max_content)
                .fold(0.0, f32::max);
            if let Some(width) = style.width {
                let outer = width.max(0.0) + horizontal_edges;
                IntrinsicSizes {
                    min_content: outer,
                    max_content: outer,
                }
            } else {
                IntrinsicSizes {
                    min_content: child_min + horizontal_edges,
                    max_content: child_max + horizontal_edges,
                }
            }
        }
    }
}

#[derive(Default)]
struct FragmentBuilder {
    next_id: usize,
}

impl FragmentBuilder {
    fn build(&mut self, tree: &LayoutTree, viewport: Size) -> FragmentTree {
        let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
        let containing_block = ContainingBlock {
            origin: Point { x: 0.0, y: 0.0 },
            available: Size {
                width: viewport.width.max(0.0),
                height: viewport.height.max(0.0),
            },
        };
        let mut cursor_y = containing_block.origin.y;
        let mut children = Vec::new();

        for child in &tree.root.children {
            children.push(self.layout_node(child, containing_block, &mut cursor_y));
        }

        FragmentTree {
            root: Fragment {
                id: self.allocate_id(),
                layout_node: tree.root.id,
                dom_node: tree.root.dom_node,
                kind: FragmentKind::Root,
                boxes: BoxModel::single(viewport_rect),
                style: tree.root.style,
                children,
            },
        }
    }

    fn layout_node(
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

    fn layout_box(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Fragment {
        let style = node.style;
        let x = containing_block.origin.x;
        let available_width = containing_block.available.width;
        let horizontal_edges = style.margin.horizontal()
            + style.border_width.horizontal()
            + style.padding.horizontal();

        let content_width = style
            .width
            .unwrap_or_else(|| (available_width - horizontal_edges).max(0.0))
            .max(0.0);

        let margin_top = *cursor_y;
        let border_x = x + style.margin.left;
        let border_y = margin_top + style.margin.top;
        let padding_x = border_x + style.border_width.left;
        let padding_y = border_y + style.border_width.top;
        let content_x = padding_x + style.padding.left;
        let content_y = padding_y + style.padding.top;

        let child_containing_block = ContainingBlock {
            origin: Point {
                x: content_x,
                y: content_y,
            },
            available: Size {
                width: content_width,
                height: containing_block.available.height,
            },
        };
        let mut child_y = child_containing_block.origin.y;
        let mut children = Vec::new();
        for child in &node.children {
            children.push(self.layout_node(child, child_containing_block, &mut child_y));
        }

        let natural_content_height = (child_y - content_y).max(0.0);
        let content_height = style.height.unwrap_or(natural_content_height).max(0.0);

        let content_box = Rect::new(content_x, content_y, content_width, content_height);
        let padding_box = Rect::new(
            padding_x,
            padding_y,
            content_width + style.padding.horizontal(),
            content_height + style.padding.vertical(),
        );
        let border_box = Rect::new(
            border_x,
            border_y,
            padding_box.size.width + style.border_width.horizontal(),
            padding_box.size.height + style.border_width.vertical(),
        );
        let margin_box = Rect::new(
            x,
            margin_top,
            border_box.size.width + style.margin.horizontal(),
            border_box.size.height + style.margin.vertical(),
        );

        *cursor_y = margin_box.origin.y + margin_box.size.height;

        Fragment {
            id: self.allocate_id(),
            layout_node: node.id,
            dom_node: node.dom_node,
            kind: FragmentKind::Box,
            boxes: BoxModel {
                margin_box,
                border_box,
                padding_box,
                content_box,
            },
            style,
            children,
        }
    }

    fn allocate_id(&mut self) -> FragmentId {
        let id = FragmentId(self.next_id);
        self.next_id += 1;
        id
    }
}

fn snapshot_layout_node(node: &LayoutNode, depth: usize, output: &mut String) {
    let dom = node
        .dom_node
        .map(|node| node.to_string())
        .unwrap_or_else(|| "-".into());
    let kind = match &node.kind {
        LayoutNodeKind::Root => "root".to_string(),
        LayoutNodeKind::Box => "box".to_string(),
        LayoutNodeKind::Text(run) => format!("text:{}", run.text),
    };
    output.push_str(&format!(
        "{}layout={}|dom={dom}|kind={kind}|children={}\n",
        " ".repeat(depth),
        node.id.index(),
        node.children.len()
    ));
    for child in &node.children {
        snapshot_layout_node(child, depth + 1, output);
    }
}

fn snapshot_style_node(node: &LayoutNode, output: &mut String) {
    if let Some(dom) = node.dom_node {
        let style = node.style;
        output.push_str(&format!(
            "dom={dom}|w={:?}|h={:?}|m={:.1},{:.1},{:.1},{:.1}|b={:.1},{:.1},{:.1},{:.1}|p={:.1},{:.1},{:.1},{:.1}|bg={:02x}{:02x}{:02x}{:02x}|bc={:02x}{:02x}{:02x}{:02x}|none={}\n",
            style.width,
            style.height,
            style.margin.top,
            style.margin.right,
            style.margin.bottom,
            style.margin.left,
            style.border_width.top,
            style.border_width.right,
            style.border_width.bottom,
            style.border_width.left,
            style.padding.top,
            style.padding.right,
            style.padding.bottom,
            style.padding.left,
            style.background.r,
            style.background.g,
            style.background.b,
            style.background.a,
            style.border_color.r,
            style.border_color.g,
            style.border_color.b,
            style.border_color.a,
            style.display_none,
        ));
    }
    for child in &node.children {
        snapshot_style_node(child, output);
    }
}

fn snapshot_fragment(fragment: &Fragment, depth: usize, output: &mut String) {
    let dom = fragment
        .dom_node
        .map(|node| node.to_string())
        .unwrap_or_else(|| "-".into());
    output.push_str(&format!(
        "{}fragment={}|layout={}|dom={dom}|kind={:?}|margin={}|border={}|padding={}|content={}\n",
        " ".repeat(depth),
        fragment.id.index(),
        fragment.layout_node.index(),
        fragment.kind,
        rect_snapshot(fragment.boxes.margin_box),
        rect_snapshot(fragment.boxes.border_box),
        rect_snapshot(fragment.boxes.padding_box),
        rect_snapshot(fragment.boxes.content_box),
    ));
    for child in &fragment.children {
        snapshot_fragment(child, depth + 1, output);
    }
}

fn rect_snapshot(rect: Rect) -> String {
    format!(
        "{:.1},{:.1},{:.1},{:.1}",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_dom::{ElementData, NodeKind};
    use std::collections::BTreeMap;

    fn element(name: &str, style: Option<&str>) -> NodeKind {
        let mut attributes = BTreeMap::new();
        if let Some(style) = style {
            attributes.insert("style".into(), style.into());
        }

        NodeKind::Element(ElementData {
            tag_name: name.into(),
            attributes,
        })
    }

    #[test]
    fn layout_identity_is_distinct_and_fragments_point_back_to_it() {
        let mut doc = Document::new();
        let element = doc.append_new(doc.root(), element("div", None)).unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let layout_node = &output.tree.root.children[0];
        let fragment = &output.fragments.root.children[0];

        assert_eq!(layout_node.dom_node, Some(element));
        assert_eq!(fragment.dom_node, Some(element));
        assert_eq!(fragment.layout_node, layout_node.id);
    }

    #[test]
    fn author_stylesheet_participates_in_layout() {
        let mut doc = Document::new();
        let style = doc.append_new(doc.root(), element("style", None)).unwrap();
        doc.append_new(style, NodeKind::Text(".card { width:42px; }".into()))
            .unwrap();
        let mut attributes = BTreeMap::new();
        attributes.insert("class".into(), "card".into());
        doc.append_new(
            doc.root(),
            NodeKind::Element(ElementData {
                tag_name: "div".into(),
                attributes,
            }),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        assert_eq!(
            output.fragments.root.children[0]
                .boxes
                .content_box
                .size
                .width,
            42.0
        );
    }

    #[test]
    fn box_model_tracks_margin_border_padding_and_content_boxes() {
        let mut doc = Document::new();
        doc.append_new(
            doc.root(),
            element(
                "div",
                Some(
                    "width:100px;height:20px;margin:5px;padding:10px;\
                     border-width:2px;border-color:#000000",
                ),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let fragment = &output.fragments.root.children[0];

        assert_eq!(fragment.boxes.margin_box, Rect::new(0.0, 0.0, 134.0, 54.0));
        assert_eq!(fragment.boxes.border_box, Rect::new(5.0, 5.0, 124.0, 44.0));
        assert_eq!(fragment.boxes.padding_box, Rect::new(7.0, 7.0, 120.0, 40.0));
        assert_eq!(
            fragment.boxes.content_box,
            Rect::new(17.0, 17.0, 100.0, 20.0)
        );
    }

    #[test]
    fn display_none_nodes_do_not_enter_the_layout_or_fragment_trees() {
        let mut doc = Document::new();
        doc.append_new(doc.root(), element("div", Some("display:none")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );

        assert!(output.tree.root.children.is_empty());
        assert!(output.fragments.root.children.is_empty());
    }

    #[test]
    fn text_runs_expose_intrinsic_sizes() {
        let run = TextRun::new("small verylongword".into());
        assert_eq!(
            run.intrinsic_sizes(),
            IntrinsicSizes {
                min_content: 96.0,
                max_content: 144.0,
            }
        );
    }

    #[test]
    fn nested_boxes_use_parent_content_as_containing_block() {
        let mut doc = Document::new();
        let parent = doc
            .append_new(doc.root(), element("div", Some("width:100px;padding:10px")))
            .unwrap();
        doc.append_new(parent, element("div", None)).unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let parent_fragment = &output.fragments.root.children[0];
        let child = &parent_fragment.children[0];
        assert_eq!(child.boxes.content_box.origin.x, 10.0);
        assert_eq!(child.boxes.content_box.size.width, 100.0);
    }

    #[test]
    fn snapshots_are_deterministic() {
        let mut doc = Document::new();
        doc.append_new(doc.root(), element("div", Some("width:20px")))
            .unwrap();
        let output = layout_document(
            &doc,
            Size {
                width: 100.0,
                height: 100.0,
            },
        );

        assert_eq!(output.tree.snapshot(), output.tree.snapshot());
        assert_eq!(output.tree.style_snapshot(), output.tree.style_snapshot());
        assert_eq!(output.fragments.snapshot(), output.fragments.snapshot());
    }
}
