use rarog_css::{ComputedStyle, computed_style};
use rarog_dom::{Document, NodeId, NodeKind};
use rarog_types::{Rect, Size};

#[derive(Clone, Debug)]
pub struct LayoutBox {
    pub node: NodeId,
    pub rect: Rect,
    pub style: ComputedStyle,
    pub children: Vec<LayoutBox>,
}

pub fn layout_document(doc: &Document, viewport: Size) -> LayoutBox {
    let mut root = LayoutBox {
        node: doc.root(),
        rect: Rect::new(0.0, 0.0, viewport.width, viewport.height),
        style: ComputedStyle::default(),
        children: Vec::new(),
    };
    let mut cursor_y = 0.0;
    for child in doc.children(doc.root()) {
        if let Some(layout) = layout_node(doc, *child, 0.0, &mut cursor_y, viewport.width) {
            root.children.push(layout);
        }
    }
    root
}

fn layout_node(doc: &Document, node: NodeId, x: f32, cursor_y: &mut f32, available_width: f32) -> Option<LayoutBox> {
    match &doc.node(node).kind {
        NodeKind::Text(text) => {
            let height = 18.0;
            let width = (text.chars().count() as f32 * 8.0).min(available_width.max(0.0));
            let rect = Rect::new(x, *cursor_y, width, height);
            *cursor_y += height;
            Some(LayoutBox { node, rect, style: ComputedStyle::default(), children: Vec::new() })
        }
        NodeKind::Document => None,
        NodeKind::Element(_) => {
            let style = computed_style(doc, node);
            if style.display_none { return None; }
            let outer_width = style.width.unwrap_or(available_width).min(available_width.max(0.0));
            let content_width = (outer_width - style.padding.horizontal()).max(0.0);
            let top = *cursor_y;
            let content_x = x + style.padding.left;
            let mut child_y = top + style.padding.top;
            let mut children = Vec::new();
            for child in doc.children(node) {
                if let Some(layout) = layout_node(doc, *child, content_x, &mut child_y, content_width) {
                    children.push(layout);
                }
            }
            let content_height = (child_y - (top + style.padding.top)).max(0.0);
            let height = style.height.unwrap_or(content_height) + style.padding.vertical();
            let rect = Rect::new(x, top, outer_width, height);
            *cursor_y = top + height;
            Some(LayoutBox { node, rect, style, children })
        }
    }
}
