from pathlib import Path

layout = Path("crates/rarog-layout/src/lib.rs")
text = layout.read_text()

anchor = '''#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntrinsicSizes {
'''
insert = '''#[derive(Clone, Copy, Debug, PartialEq)]
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
        let clusters = text
            .chars()
            .enumerate()
            .map(|(index, _)| GlyphCluster {
                source: TextRange::new(index, index + 1),
                advance: self.advance,
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
'''
if anchor not in text:
    raise SystemExit("intrinsic anchor not found")
text = text.replace(anchor, insert, 1)

old_run = '''#[derive(Clone, Debug, PartialEq)]
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

    pub fn character_count(&self) -> usize {
        self.text.chars().count()
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
'''
new_run = '''#[derive(Clone, Debug, PartialEq)]
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
        self.shaped.clusters.len()
    }

    pub fn advance_for_range(&self, range: TextRange) -> f32 {
        self.shaped
            .clusters
            .iter()
            .filter(|cluster| cluster.source.start >= range.start && cluster.source.end <= range.end)
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
'''
if old_run not in text:
    raise SystemExit("text run marker not found")
text = text.replace(old_run, new_run, 1)

old_breaker = '''#[derive(Clone, Copy, Debug, PartialEq)]
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
'''
new_breaker = '''#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedAdvanceLineBreaker;

impl LineBreaker for FixedAdvanceLineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange> {
        if run.shaped.clusters.is_empty() {
            return vec![TextRange::new(0, 0)];
        }
        let first_advance = run.shaped.clusters[0].advance.max(f32::EPSILON);
        if available_width < first_advance {
            return vec![TextRange::new(0, run.character_count())];
        }
        let mut ranges = Vec::new();
        let mut start = 0;
        let mut width = 0.0;
        for cluster in &run.shaped.clusters {
            if cluster.source.start > start && width + cluster.advance > available_width {
                ranges.push(TextRange::new(start, cluster.source.start));
                start = cluster.source.start;
                width = 0.0;
            }
            width += cluster.advance;
        }
        ranges.push(TextRange::new(start, run.character_count()));
        ranges
    }
}
'''
if old_breaker not in text:
    raise SystemExit("line breaker marker not found")
text = text.replace(old_breaker, new_breaker, 1)

old_width = '''            let width = (text_range.len() as f32 * line_breaker.advance).min(available_width);
'''
new_width = '''            let width = run.advance_for_range(text_range).min(available_width);
'''
if old_width not in text:
    raise SystemExit("line width marker not found")
text = text.replace(old_width, new_width, 1)

needle = '''    fn fixed_advance_line_breaker_returns_stable_text_ranges() {
        let breaker = FixedAdvanceLineBreaker::default();
        let run = TextRun::new("abcdefg".into());
'''
if needle not in text:
    raise SystemExit("breaker test marker not found")

module_end = text.rfind("\n}")
extra = r'''

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
        run.advance = run.shaped.clusters.iter().map(|cluster| cluster.advance).sum();
        let breaker = FixedAdvanceLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 16.0),
            vec![TextRange::new(0, 1), TextRange::new(1, 2), TextRange::new(2, 3)]
        );
    }
'''
text = text[:module_end] + extra + text[module_end:]
layout.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
text = architecture.read_text()
anchor = "Text fragmentation now records explicit source-character `TextRange` values"
pos = text.find(anchor)
if pos < 0:
    raise SystemExit("architecture marker not found")
end = text.find("\n\n", pos)
addition = "\n\nText measurement is separated from layout through `TextShaper`, `ShapedText`, `GlyphCluster`, and `FontMetrics`. The bootstrap shaper emits one fixed-advance cluster per source character, while line breaking consumes cluster advances rather than assuming character width. This keeps shaping/font selection replaceable and makes variable-width or multi-codepoint clusters possible without redesigning the fragment contract."
text = text[:end] + addition + text[end:]
architecture.write_text(text)

adr = Path("docs/adr/0016-text-shaping-boundary.md")
adr.write_text("""# ADR-0016: Text shaping boundary

## Status

Accepted.

## Context

Line boxes and text ranges need measurement data, but layout must not depend directly on a specific font or shaping library.

## Decision

Introduce `TextShaper`, `ShapedText`, `GlyphCluster`, and `FontMetrics` in the layout-facing text model. The R0 bootstrap shaper is deterministic and fixed-advance. Line breaking consumes shaped cluster advances and source ranges.

No external font or shaping backend is selected by this decision.

## Consequences

A real shaping implementation can later provide variable advances, multi-character clusters, font metrics, bidi-aware ordering, and font fallback behind the same boundary. Layout and retained-paint identity remain independent of the concrete backend.
""")
