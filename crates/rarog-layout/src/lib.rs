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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FragmentOrdinal(u32);

impl FragmentOrdinal {
    pub const fn index(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineBox {
    pub ordinal: u32,
    pub rect: Rect,
    pub text_range: TextRange,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

impl FontMetrics {
    pub fn line_height(self) -> f32 {
        self.ascent + self.descent + self.line_gap
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphCluster {
    pub source: TextRange,
    pub advance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedText {
    pub clusters: Vec<GlyphCluster>,
    pub advance: f32,
    pub metrics: FontMetrics,
}

pub trait TextShaper {
    fn shape(&self, text: &str) -> ShapedText;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedTextShaper {
    pub advance: f32,
    pub metrics: FontMetrics,
}

impl Default for FixedTextShaper {
    fn default() -> Self {
        Self {
            advance: 8.0,
            metrics: FontMetrics {
                ascent: 14.0,
                descent: 4.0,
                line_gap: 0.0,
            },
        }
    }
}

impl TextShaper for FixedTextShaper {
    fn shape(&self, text: &str) -> ShapedText {
        let characters = text.chars().collect::<Vec<_>>();
        let boundaries = grapheme_boundaries(text);
        let clusters = boundaries
            .windows(2)
            .map(|window| {
                let start = window[0];
                let end = window[1];
                let mandatory = characters[start..end]
                    .iter()
                    .copied()
                    .any(is_mandatory_break);
                GlyphCluster {
                    source: TextRange::new(start, end),
                    advance: if mandatory { 0.0 } else { self.advance },
                }
            })
            .collect::<Vec<_>>();
        ShapedText {
            advance: clusters.iter().map(|cluster| cluster.advance).sum(),
            clusters,
            metrics: self.metrics,
        }
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
    pub shaped: ShapedText,
    pub advance: f32,
    pub line_height: f32,
}

impl TextRun {
    pub fn new(text: String) -> Self {
        let shaper = FixedTextShaper::default();
        let shaped = shaper.shape(&text);
        Self {
            text,
            advance: shaped.advance,
            line_height: shaped.metrics.line_height(),
            shaped,
        }
    }

    pub fn character_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn advance_for_range(&self, range: TextRange) -> f32 {
        self.shaped
            .clusters
            .iter()
            .filter(|cluster| {
                cluster.source.start >= range.start && cluster.source.end <= range.end
            })
            .map(|cluster| cluster.advance)
            .sum()
    }

    pub fn intrinsic_sizes(&self) -> IntrinsicSizes {
        let shaper = FixedTextShaper::default();
        let longest_word = self
            .text
            .split_whitespace()
            .map(|word| shaper.shape(word).advance)
            .fold(0.0, f32::max);
        IntrinsicSizes {
            min_content: longest_word,
            max_content: self.advance,
        }
    }
}

pub trait LineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakKind {
    Soft,
    Mandatory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakOpportunity {
    pub index: usize,
    pub kind: BreakKind,
}

pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut boundaries = vec![0];
    for index in 1..characters.len() {
        let previous = characters[index - 1];
        let current = characters[index];
        let previous_previous = index.checked_sub(2).map(|value| characters[value]);
        let preceding_regional_indicators = characters[..index]
            .iter()
            .rev()
            .take_while(|character| is_regional_indicator(**character))
            .count();

        let no_break = (previous == '\r' && current == '\n')
            || is_grapheme_extend(current)
            || previous == '\u{200d}'
            || current == '\u{200d}'
            || (is_regional_indicator(previous)
                && is_regional_indicator(current)
                && preceding_regional_indicators % 2 == 1)
            || (previous_previous == Some('\u{200d}') && is_extended_pictographic(current));

        if !no_break {
            boundaries.push(index);
        }
    }

    boundaries.push(characters.len());
    boundaries.dedup();
    boundaries
}

pub fn is_grapheme_boundary(text: &str, index: usize) -> bool {
    grapheme_boundaries(text).binary_search(&index).is_ok()
}

fn is_grapheme_extend(character: char) -> bool {
    matches!(
        character as u32,
        0x0300..=0x036f
            | 0x0483..=0x0489
            | 0x0591..=0x05bd
            | 0x05bf
            | 0x05c1..=0x05c2
            | 0x05c4..=0x05c5
            | 0x0610..=0x061a
            | 0x064b..=0x065f
            | 0x0670
            | 0x06d6..=0x06ed
            | 0x1ab0..=0x1aff
            | 0x1dc0..=0x1dff
            | 0x20d0..=0x20ff
            | 0xfe00..=0xfe0f
            | 0xfe20..=0xfe2f
            | 0xe0100..=0xe01ef
            | 0x1f3fb..=0x1f3ff
    )
}

fn is_regional_indicator(character: char) -> bool {
    matches!(character as u32, 0x1f1e6..=0x1f1ff)
}

fn is_extended_pictographic(character: char) -> bool {
    matches!(character as u32, 0x1f000..=0x1faff | 0x2600..=0x27bf)
}

pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut opportunities = Vec::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let boundary = index + 1;
        if is_mandatory_break(character) {
            if is_grapheme_boundary(text, boundary) {
                opportunities.push(BreakOpportunity {
                    index: boundary,
                    kind: BreakKind::Mandatory,
                });
            }
            continue;
        }
        let next = characters.get(boundary).copied();
        if is_grapheme_boundary(text, boundary)
            && (is_breakable_whitespace(character)
                || character == '-'
                || (is_cjk_ideograph(character) && next.is_some_and(is_cjk_ideograph)))
        {
            opportunities.push(BreakOpportunity {
                index: boundary,
                kind: BreakKind::Soft,
            });
        }
    }
    opportunities
}

fn is_mandatory_break(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn is_breakable_whitespace(character: char) -> bool {
    character.is_whitespace()
        && !is_mandatory_break(character)
        && !matches!(character, '\u{00a0}' | '\u{202f}')
}

fn is_cjk_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
    )
}

fn is_non_breaking_boundary(text: &str, index: usize) -> bool {
    let characters = text.chars().collect::<Vec<_>>();
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index);
    previous
        .into_iter()
        .chain(next)
        .any(|character| matches!(character, '\u{00a0}' | '\u{202f}'))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnicodeLineBreaker;

impl LineBreaker for UnicodeLineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange> {
        if run.shaped.clusters.is_empty() {
            return vec![TextRange::new(0, 0)];
        }

        let opportunities = unicode_break_opportunities(&run.text);
        let mut ranges = Vec::new();
        let mut line_start = 0;
        let mut last_soft = None;
        let mut width = 0.0;

        for cluster in &run.shaped.clusters {
            width += cluster.advance;
            let boundary = cluster.source.end;
            let opportunity = opportunities
                .iter()
                .find(|opportunity| opportunity.index == boundary)
                .copied();

            if matches!(
                opportunity.map(|value| value.kind),
                Some(BreakKind::Mandatory)
            ) {
                ranges.push(TextRange::new(line_start, boundary));
                line_start = boundary;
                last_soft = None;
                width = 0.0;
                continue;
            }

            if available_width.is_finite() && available_width >= 0.0 && width > available_width {
                let emergency = cluster.source.start;
                let break_at = last_soft.filter(|value| *value > line_start).or_else(|| {
                    (emergency > line_start
                        && is_grapheme_boundary(&run.text, emergency)
                        && !is_non_breaking_boundary(&run.text, emergency))
                    .then_some(emergency)
                });
                if let Some(break_at) = break_at {
                    ranges.push(TextRange::new(line_start, break_at));
                    line_start = break_at;
                    width = run.advance_for_range(TextRange::new(line_start, boundary));
                    last_soft = opportunities
                        .iter()
                        .filter(|value| {
                            value.kind == BreakKind::Soft
                                && value.index > line_start
                                && value.index < boundary
                        })
                        .map(|value| value.index)
                        .next_back();
                }
            }

            if matches!(opportunity.map(|value| value.kind), Some(BreakKind::Soft))
                && boundary > line_start
            {
                last_soft = Some(boundary);
            }
        }

        if line_start < run.character_count() {
            ranges.push(TextRange::new(line_start, run.character_count()));
        }
        if ranges.is_empty() {
            ranges.push(TextRange::new(0, 0));
        }
        ranges
    }
}

pub type FixedAdvanceLineBreaker = UnicodeLineBreaker;

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
    pub ordinal: FragmentOrdinal,
    pub layout_node: LayoutNodeId,
    pub dom_node: Option<NodeId>,
    pub kind: FragmentKind,
    pub boxes: BoxModel,
    pub style: ComputedStyle,
    pub text_range: Option<TextRange>,
    pub line_box: Option<LineBox>,
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

pub fn fragments_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Vec<&Fragment> {
    let mut fragments = Vec::new();
    collect_fragments(&tree.root, dom_node, &mut fragments);
    fragments
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
        rebuilt.extend(builder.layout_node(child, containing_block, &mut cursor_y));
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

fn collect_fragments<'a>(fragment: &'a Fragment, dom_node: NodeId, output: &mut Vec<&'a Fragment>) {
    if fragment.dom_node == Some(dom_node) {
        output.push(fragment);
    }
    for child in &fragment.children {
        collect_fragments(child, dom_node, output);
    }
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
            let mut replacement = builder.layout_node(layout_node, containing_block, &mut cursor_y);
            if replacement.len() != 1 {
                return false;
            }
            *child = replacement.remove(0);
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
            children.extend(self.layout_node(child, containing_block, &mut cursor_y));
        }

        FragmentTree {
            root: Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(0),
                layout_node: tree.root.id,
                dom_node: tree.root.dom_node,
                kind: FragmentKind::Root,
                boxes: BoxModel::single(viewport_rect),
                style: tree.root.style,
                text_range: None,
                line_box: None,
                children,
            },
        }
    }

    fn layout_node(
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
        let available_width = containing_block.available.width.max(0.0);
        let line_breaker = UnicodeLineBreaker;
        let ranges = line_breaker.break_text(run, available_width);
        let mut fragments = Vec::with_capacity(ranges.len());
        for (ordinal, text_range) in ranges.into_iter().enumerate() {
            let width = run.advance_for_range(text_range).min(available_width);
            let rect = Rect::new(containing_block.origin.x, *cursor_y, width, run.line_height);
            *cursor_y += run.line_height;
            let line_box = LineBox {
                ordinal: ordinal as u32,
                rect,
                text_range,
            };
            fragments.push(Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(ordinal as u32),
                layout_node: node.id,
                dom_node: node.dom_node,
                kind: FragmentKind::Text,
                boxes: BoxModel::single(rect),
                style: node.style,
                text_range: Some(text_range),
                line_box: Some(line_box),
                children: Vec::new(),
            });
        }
        fragments
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
            children.extend(self.layout_node(child, child_containing_block, &mut child_y));
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
            ordinal: FragmentOrdinal(0),
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
            text_range: None,
            line_box: None,
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
    let text_range = fragment
        .text_range
        .map(|range| format!("{}..{}", range.start, range.end))
        .unwrap_or_else(|| "-".into());
    let line = fragment
        .line_box
        .map(|line| format!("{}:{}", line.ordinal, rect_snapshot(line.rect)))
        .unwrap_or_else(|| "-".into());
    output.push_str(&format!(
        "{}fragment={}|ordinal={}|layout={}|dom={dom}|kind={:?}|range={text_range}|line={line}|margin={}|border={}|padding={}|content={}\n",
        " ".repeat(depth),
        fragment.id.index(),
        fragment.ordinal.index(),
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
        assert!(
            fragments
                .iter()
                .all(|fragment| fragment.layout_node == layout_node.id)
        );
        assert_eq!(
            fragments
                .iter()
                .map(|fragment| fragment.ordinal.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(fragments[0].boxes.content_box.size.width, 24.0);
        assert_eq!(fragments[3].boxes.content_box.size.width, 8.0);
        assert_eq!(fragments[0].text_range, Some(TextRange::new(0, 3)));
        assert_eq!(fragments[1].text_range, Some(TextRange::new(3, 6)));
        assert_eq!(fragments[2].text_range, Some(TextRange::new(6, 9)));
        assert_eq!(fragments[3].text_range, Some(TextRange::new(9, 10)));
        assert_eq!(fragments[0].line_box.unwrap().ordinal, 0);
        assert_eq!(fragments[3].line_box.unwrap().ordinal, 3);
    }

    #[test]
    fn fixed_advance_line_breaker_returns_stable_text_ranges() {
        let breaker = UnicodeLineBreaker;
        let run = TextRun::new("abcdefg".into());
        assert_eq!(
            breaker.break_text(&run, 24.0),
            vec![
                TextRange::new(0, 3),
                TextRange::new(3, 6),
                TextRange::new(6, 7),
            ]
        );
    }

    #[test]
    fn fixed_text_shaper_exposes_clusters_and_font_metrics() {
        let shaper = FixedTextShaper::default();
        let shaped = shaper.shape("abc");
        assert_eq!(shaped.advance, 24.0);
        assert_eq!(shaped.metrics.line_height(), 18.0);
        assert_eq!(shaped.clusters.len(), 3);
        assert_eq!(shaped.clusters[0].source, TextRange::new(0, 1));
        assert_eq!(shaped.clusters[2].source, TextRange::new(2, 3));
    }

    #[test]
    fn line_breaker_consumes_shaped_cluster_advances() {
        let mut run = TextRun::new("abc".into());
        run.shaped.clusters[1].advance = 16.0;
        run.advance = run
            .shaped
            .clusters
            .iter()
            .map(|cluster| cluster.advance)
            .sum();
        let breaker = UnicodeLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 16.0),
            vec![
                TextRange::new(0, 1),
                TextRange::new(1, 2),
                TextRange::new(2, 3)
            ]
        );
    }

    #[test]
    fn unicode_break_opportunities_cover_whitespace_hyphen_cjk_and_mandatory_breaks() {
        assert_eq!(
            unicode_break_opportunities("a b-c中日\nq"),
            vec![
                BreakOpportunity {
                    index: 2,
                    kind: BreakKind::Soft
                },
                BreakOpportunity {
                    index: 4,
                    kind: BreakKind::Soft
                },
                BreakOpportunity {
                    index: 6,
                    kind: BreakKind::Soft
                },
                BreakOpportunity {
                    index: 8,
                    kind: BreakKind::Mandatory
                },
            ]
        );
    }

    #[test]
    fn unicode_line_breaker_prefers_legal_soft_breaks() {
        let run = TextRun::new("hello world".into());
        let breaker = UnicodeLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 48.0),
            vec![TextRange::new(0, 6), TextRange::new(6, 11)]
        );
    }

    #[test]
    fn unicode_line_breaker_preserves_non_breaking_spaces() {
        let run = TextRun::new("a\u{00a0}b".into());
        let breaker = UnicodeLineBreaker;
        assert_eq!(breaker.break_text(&run, 8.0), vec![TextRange::new(0, 3)]);
    }

    #[test]
    fn unicode_line_breaker_honors_mandatory_breaks() {
        let run = TextRun::new("ab\ncd".into());
        let breaker = UnicodeLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 200.0),
            vec![TextRange::new(0, 3), TextRange::new(3, 5)]
        );
        assert_eq!(run.shaped.clusters[2].advance, 0.0);
    }

    #[test]
    fn unicode_line_breaker_allows_cjk_breaks_without_spaces() {
        let run = TextRun::new("中文测试".into());
        let breaker = UnicodeLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 16.0),
            vec![TextRange::new(0, 2), TextRange::new(2, 4)]
        );
    }

    #[test]
    fn grapheme_boundaries_keep_combining_sequences_together() {
        let text = "e\u{301}x";
        assert_eq!(grapheme_boundaries(text), vec![0, 2, 3]);
        let run = TextRun::new(text.into());
        assert_eq!(run.shaped.clusters[0].source, TextRange::new(0, 2));
        assert_eq!(run.shaped.clusters.len(), 2);
    }

    #[test]
    fn grapheme_boundaries_keep_emoji_modifier_and_zwj_sequences_together() {
        let text = "👩🏽\u{200d}💻x";
        assert_eq!(grapheme_boundaries(text), vec![0, 4, 5]);
        let run = TextRun::new(text.into());
        assert_eq!(run.shaped.clusters[0].source, TextRange::new(0, 4));
    }

    #[test]
    fn grapheme_boundaries_pair_regional_indicators() {
        let text = "🇺🇸🇨🇦";
        assert_eq!(grapheme_boundaries(text), vec![0, 2, 4]);
    }

    #[test]
    fn unicode_line_breaker_never_emergency_breaks_inside_grapheme_cluster() {
        let run = TextRun::new("e\u{301}x".into());
        let breaker = UnicodeLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 4.0),
            vec![TextRange::new(0, 2), TextRange::new(2, 3)]
        );
    }

    #[test]
    fn crlf_is_one_grapheme_cluster_and_one_mandatory_boundary() {
        let text = "a\r\nb";
        assert_eq!(grapheme_boundaries(text), vec![0, 1, 3, 4]);
        assert_eq!(
            unicode_break_opportunities(text)
                .into_iter()
                .filter(|value| value.kind == BreakKind::Mandatory)
                .collect::<Vec<_>>(),
            vec![BreakOpportunity {
                index: 3,
                kind: BreakKind::Mandatory
            }]
        );
    }
}
