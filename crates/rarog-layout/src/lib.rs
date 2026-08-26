use rarog_css::{ComputedStyle, computed_style};
use rarog_dom::{Document, NodeId, NodeKind};
use rarog_types::{Rect, Size};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutNodeKind {
    Root,
    Box,
    Text(String),
}

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub id: LayoutNodeId,
    pub dom_node: Option<NodeId>,
    pub kind: LayoutNodeKind,
    pub style: ComputedStyle,
    pub children: Vec<LayoutNode>,
}

#[derive(Clone, Debug)]
pub struct LayoutTree {
    pub root: LayoutNode,
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

#[derive(Clone, Debug)]
pub struct LayoutOutput {
    pub tree: LayoutTree,
    pub fragments: FragmentTree,
}

pub fn layout_document(doc: &Document, viewport: Size) -> LayoutOutput {
    let mut tree_builder = LayoutTreeBuilder::default();
    let root = tree_builder
        .build_node(doc, doc.root())
        .expect("document root always creates a layout root");
    let tree = LayoutTree { root };

    let mut fragment_builder = FragmentBuilder::default();
    let fragments = fragment_builder.build(&tree, viewport);

    LayoutOutput { tree, fragments }
}

#[derive(Default)]
struct LayoutTreeBuilder {
    next_id: usize,
}

impl LayoutTreeBuilder {
    fn build_node(&mut self, doc: &Document, node: NodeId) -> Option<LayoutNode> {
        let (kind, style) = match &doc.node(node).kind {
            NodeKind::Document => (LayoutNodeKind::Root, ComputedStyle::default()),
            NodeKind::Text(text) => (
                LayoutNodeKind::Text(text.clone()),
                ComputedStyle::default(),
            ),
            NodeKind::Element(_) => {
                let style = computed_style(doc, node);
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

        Some(LayoutNode {
            id,
            dom_node: Some(node),
            kind,
            style,
            children,
        })
    }

    fn allocate_id(&mut self) -> LayoutNodeId {
        let id = LayoutNodeId(self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Default)]
struct FragmentBuilder {
    next_id: usize,
}

impl FragmentBuilder {
    fn build(&mut self, tree: &LayoutTree, viewport: Size) -> FragmentTree {
        let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
        let mut cursor_y = 0.0;
        let mut children = Vec::new();

        for child in &tree.root.children {
            children.push(self.layout_node(
                child,
                0.0,
                &mut cursor_y,
                viewport.width.max(0.0),
            ));
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
        x: f32,
        cursor_y: &mut f32,
        available_width: f32,
    ) -> Fragment {
        match &node.kind {
            LayoutNodeKind::Root => unreachable!("only the layout root may have Root kind"),
            LayoutNodeKind::Text(text) => {
                self.layout_text(node, text, x, cursor_y, available_width)
            }
            LayoutNodeKind::Box => self.layout_box(node, x, cursor_y, available_width),
        }
    }

    fn layout_text(
        &mut self,
        node: &LayoutNode,
        text: &str,
        x: f32,
        cursor_y: &mut f32,
        available_width: f32,
    ) -> Fragment {
        let height = 18.0;
        let width = (text.chars().count() as f32 * 8.0).min(available_width.max(0.0));
        let rect = Rect::new(x, *cursor_y, width, height);
        *cursor_y += height;

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
        x: f32,
        cursor_y: &mut f32,
        available_width: f32,
    ) -> Fragment {
        let style = node.style;
        let horizontal_edges =
            style.margin.horizontal() + style.border_width.horizontal() + style.padding.horizontal();

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

        let mut child_y = content_y;
        let mut children = Vec::new();
        for child in &node.children {
            children.push(self.layout_node(child, content_x, &mut child_y, content_width));
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

        assert_eq!(
            fragment.boxes.margin_box,
            Rect::new(0.0, 0.0, 134.0, 54.0)
        );
        assert_eq!(
            fragment.boxes.border_box,
            Rect::new(5.0, 5.0, 124.0, 44.0)
        );
        assert_eq!(
            fragment.boxes.padding_box,
            Rect::new(7.0, 7.0, 120.0, 40.0)
        );
        assert_eq!(
            fragment.boxes.content_box,
            Rect::new(17.0, 17.0, 100.0, 20.0)
        );
    }

    #[test]
    fn display_none_nodes_do_not_enter_the_layout_or_fragment_trees() {
        let mut doc = Document::new();
        doc.append_new(
            doc.root(),
            element("div", Some("display:none")),
        )
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
}
