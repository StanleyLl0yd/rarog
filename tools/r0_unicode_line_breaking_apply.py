from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text()

old = '''#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
new = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakKind {
    Soft,
    Mandatory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakOpportunity {
    pub index: usize,
    pub kind: BreakKind,
}

pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut opportunities = Vec::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let boundary = index + 1;
        if is_mandatory_break(character) {
            opportunities.push(BreakOpportunity {
                index: boundary,
                kind: BreakKind::Mandatory,
            });
            continue;
        }
        let next = characters.get(boundary).copied();
        if is_breakable_whitespace(character)
            || character == '-'
            || (is_cjk_ideograph(character) && next.is_some_and(is_cjk_ideograph))
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
    matches!(character, '\\n' | '\\r' | '\\u{2028}' | '\\u{2029}')
}

fn is_breakable_whitespace(character: char) -> bool {
    character.is_whitespace()
        && !is_mandatory_break(character)
        && !matches!(character, '\\u{00a0}' | '\\u{202f}')
}

fn is_cjk_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
    )
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

            if matches!(opportunity.map(|value| value.kind), Some(BreakKind::Mandatory)) {
                ranges.push(TextRange::new(line_start, boundary));
                line_start = boundary;
                last_soft = None;
                width = 0.0;
                continue;
            }

            if matches!(opportunity.map(|value| value.kind), Some(BreakKind::Soft)) {
                last_soft = Some(boundary);
            }

            if available_width.is_finite()
                && available_width >= 0.0
                && width > available_width
                && let Some(break_at) = last_soft
                && break_at > line_start
            {
                ranges.push(TextRange::new(line_start, break_at));
                line_start = break_at;
                width = run.advance_for_range(TextRange::new(line_start, boundary));
                last_soft = opportunities
                    .iter()
                    .filter(|value| {
                        value.kind == BreakKind::Soft
                            && value.index > line_start
                            && value.index <= boundary
                    })
                    .map(|value| value.index)
                    .last();
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
'''
if old not in text:
    raise SystemExit("line breaker block not found")
text = text.replace(old, new, 1)

text = text.replace(
    "let line_breaker = FixedAdvanceLineBreaker;",
    "let line_breaker = UnicodeLineBreaker;",
)

old_shape = '''            .map(|(index, _)| GlyphCluster {
                source: TextRange::new(index, index + 1),
                advance: self.advance,
            })
'''
new_shape = '''            .map(|(index, character)| GlyphCluster {
                source: TextRange::new(index, index + 1),
                advance: if is_mandatory_break(character) {
                    0.0
                } else {
                    self.advance
                },
            })
'''
if old_shape not in text:
    raise SystemExit("shaper cluster block not found")
text = text.replace(old_shape, new_shape, 1)

module_end = text.rfind("\n}")
extra = r'''

    #[test]
    fn unicode_break_opportunities_cover_whitespace_hyphen_cjk_and_mandatory_breaks() {
        assert_eq!(
            unicode_break_opportunities("a b-c中日\nq"),
            vec![
                BreakOpportunity { index: 2, kind: BreakKind::Soft },
                BreakOpportunity { index: 4, kind: BreakKind::Soft },
                BreakOpportunity { index: 6, kind: BreakKind::Soft },
                BreakOpportunity { index: 8, kind: BreakKind::Mandatory },
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
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
arch = architecture.read_text()
anchor = "Text measurement is separated from layout through `TextShaper`"
pos = arch.find(anchor)
if pos < 0:
    raise SystemExit("architecture marker not found")
end = arch.find("\n\n", pos)
addition = "\n\nLine breaking now consumes explicit Unicode-aware break opportunities. R0 recognizes mandatory Unicode separators, breakable Unicode whitespace, hyphen opportunities, non-breaking spaces, and basic CJK ideographic boundaries. This is intentionally a deterministic UAX #14-oriented bootstrap subset, not a claim of full Unicode Line Breaking Algorithm conformance."
arch = arch[:end] + addition + arch[end:]
architecture.write_text(arch)

backlog = Path("docs/R0-BACKLOG.md")
backlog_text = backlog.read_text()
for old_item, new_item in [
    ("- [ ] Unicode-aware line breaking", "- [x] Unicode-aware line-breaking foundation with deterministic break opportunities"),
    ("- [ ] Unicode line breaking", "- [x] Unicode line-breaking foundation with deterministic break opportunities"),
]:
    if old_item in backlog_text:
        backlog_text = backlog_text.replace(old_item, new_item, 1)
backlog.write_text(backlog_text)

adr = Path("docs/adr/0017-unicode-line-breaking-foundation.md")
adr.write_text("""# ADR-0017: Unicode line-breaking foundation

## Status

Accepted.

## Context

The initial line breaker consumed shaped advances but could only split text by width. It had no representation of legal or mandatory text boundaries.

## Decision

Introduce explicit `BreakOpportunity` values and a `UnicodeLineBreaker`. R0 recognizes mandatory line separators, breakable Unicode whitespace, hyphen opportunities, non-breaking spaces, and basic CJK ideographic boundaries. Mandatory separators receive zero advance from the bootstrap shaper.

This is a deterministic UAX #14-oriented subset. It is not full UAX #14 conformance and does not yet implement language tailoring, grapheme-boundary protection, CSS `line-break`, `word-break`, `overflow-wrap`, or hyphenation.

## Consequences

Line layout now separates shaping widths from break policy. A standards-complete Unicode line-break implementation can replace the bootstrap classifier without changing `TextRange`, `GlyphCluster`, line-box, fragment, or retained-paint identity contracts.
""")
