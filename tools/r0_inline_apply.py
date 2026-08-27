from pathlib import Path

layout = Path("crates/rarog-layout/src/lib.rs")
text = layout.read_text()

text = text.replace(
'''impl FragmentOrdinal {
    pub const fn index(self) -> u32 {
        self.0
    }
}
''',
'''impl FragmentOrdinal {
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
''', 1)

text = text.replace(
'''impl TextRun {
    pub fn new(text: String) -> Self {
''',
'''impl TextRun {
    pub fn new(text: String) -> Self {
''', 1)

marker = '''    pub fn intrinsic_sizes(&self) -> IntrinsicSizes {
'''
text = text.replace(marker,
'''    pub fn character_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn intrinsic_sizes(&self) -> IntrinsicSizes {
''', 1)

text = text.replace(
'''#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainingBlock {
''',
'''pub trait LineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedAdvanceLineBreaker {
    pub advance: f32,
}

impl Default for FixedAdvanceLineBreaker {
    fn default() -> Self {
        Self { advance: 8.0 }
    }
}

impl LineBreaker for FixedAdvanceLineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange> {
        let character_count = run.character_count();
        if character_count == 0 {
            return vec![TextRange::new(0, 0)];
        }
        let advance = self.advance.max(f32::EPSILON);
        let characters_per_line = if available_width >= advance {
            (available_width / advance).floor().max(1.0) as usize
        } else {
            character_count
        };
        let mut ranges = Vec::new();
        let mut start = 0;
        while start < character_count {
            let end = (start + characters_per_line).min(character_count);
            ranges.push(TextRange::new(start, end));
            start = end;
        }
        ranges
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainingBlock {
''', 1)

text = text.replace(
'''    pub style: ComputedStyle,
    pub children: Vec<Fragment>,
}
''',
'''    pub style: ComputedStyle,
    pub text_range: Option<TextRange>,
    pub line_box: Option<LineBox>,
    pub children: Vec<Fragment>,
}
''', 1)

text = text.replace(
'''                style: tree.root.style,
                children,
''',
'''                style: tree.root.style,
                text_range: None,
                line_box: None,
                children,
''', 1)

old = '''    fn layout_text(
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
new = '''    fn layout_text(
        &mut self,
        node: &LayoutNode,
        run: &TextRun,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Vec<Fragment> {
        let available_width = containing_block.available.width.max(0.0);
        let line_breaker = FixedAdvanceLineBreaker::default();
        let ranges = line_breaker.break_text(run, available_width);
        let mut fragments = Vec::with_capacity(ranges.len());
        for (ordinal, text_range) in ranges.into_iter().enumerate() {
            let width = (text_range.len() as f32 * line_breaker.advance).min(available_width);
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
'''
if old not in text:
    raise SystemExit("layout_text marker not found")
text = text.replace(old, new, 1)

text = text.replace(
'''            style,
            children,
        }
''',
'''            style,
            text_range: None,
            line_box: None,
            children,
        }
''', 1)

old_snapshot = '''    output.push_str(&format!(
        "{}fragment={}|ordinal={}|layout={}|dom={dom}|kind={:?}|margin={}|border={}|padding={}|content={}\\n",
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
'''
new_snapshot = '''    let text_range = fragment
        .text_range
        .map(|range| format!("{}..{}", range.start, range.end))
        .unwrap_or_else(|| "-".into());
    let line = fragment
        .line_box
        .map(|line| format!("{}:{}", line.ordinal, rect_snapshot(line.rect)))
        .unwrap_or_else(|| "-".into());
    output.push_str(&format!(
        "{}fragment={}|ordinal={}|layout={}|dom={dom}|kind={:?}|range={text_range}|line={line}|margin={}|border={}|padding={}|content={}\\n",
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
'''
if old_snapshot not in text:
    raise SystemExit("snapshot marker not found")
text = text.replace(old_snapshot, new_snapshot, 1)

needle = '''        assert_eq!(fragments[0].boxes.content_box.size.width, 24.0);
        assert_eq!(fragments[3].boxes.content_box.size.width, 8.0);
    }
'''
replacement = '''        assert_eq!(fragments[0].boxes.content_box.size.width, 24.0);
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
        let breaker = FixedAdvanceLineBreaker::default();
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
'''
if needle not in text:
    raise SystemExit("fragmentation test marker not found")
text = text.replace(needle, replacement, 1)
layout.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
text = architecture.read_text()
anchor = "Fragment identity is explicitly one-to-many with layout identity."
pos = text.find(anchor)
if pos < 0:
    raise SystemExit("architecture marker not found")
end = text.find("\n\n", pos)
addition = "\n\nText fragmentation now records explicit source-character `TextRange` values and `LineBox` geometry. Line breaking is isolated behind the `LineBreaker` abstraction; R0 uses a deterministic fixed-advance implementation so future shaping, font metrics, bidi, and standards line breaking can replace policy without changing fragment identity or retained-paint contracts."
text = text[:end] + addition + text[end:]
architecture.write_text(text)

backlog = Path("docs/R0-BACKLOG.md")
text = backlog.read_text()
for old, new in [
    ("- [ ] line box representation", "- [x] line box representation with deterministic text ranges"),
    ("- [ ] line breaking abstraction", "- [x] line breaking abstraction with fixed-advance bootstrap policy"),
]:
    if old in text:
        text = text.replace(old, new, 1)
backlog.write_text(text)
