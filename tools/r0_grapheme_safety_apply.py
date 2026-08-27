from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text()

old = '''impl TextShaper for FixedTextShaper {
    fn shape(&self, text: &str) -> ShapedText {
        let clusters = text
            .chars()
            .enumerate()
            .map(|(index, character)| GlyphCluster {
                source: TextRange::new(index, index + 1),
                advance: if is_mandatory_break(character) {
                    0.0
                } else {
                    self.advance
                },
            })
            .collect::<Vec<_>>();
        ShapedText {
            advance: clusters.iter().map(|cluster| cluster.advance).sum(),
            clusters,
            metrics: self.metrics,
        }
    }
}
'''
new = '''impl TextShaper for FixedTextShaper {
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
'''
if old not in text:
    raise SystemExit("fixed shaper block not found")
text = text.replace(old, new, 1)

text = text.replace(
    '''    pub fn character_count(&self) -> usize {
        self.shaped.clusters.len()
    }
''',
    '''    pub fn character_count(&self) -> usize {
        self.text.chars().count()
    }
''',
    1,
)

anchor = '''pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
'''
helpers = r'''pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut boundaries = vec![0];
    let mut regional_run = 0usize;

    for index in 1..characters.len() {
        let previous = characters[index - 1];
        let current = characters[index];
        let previous_previous = index.checked_sub(2).map(|value| characters[value]);

        let no_break = (previous == '\r' && current == '\n')
            || is_grapheme_extend(current)
            || previous == '\u{200d}'
            || current == '\u{200d}'
            || (is_regional_indicator(previous)
                && is_regional_indicator(current)
                && regional_run % 2 == 1)
            || (previous_previous == Some('\u{200d}') && is_extended_pictographic(current));

        if !no_break {
            boundaries.push(index);
        }

        if is_regional_indicator(current) {
            regional_run = if is_regional_indicator(previous) {
                regional_run + 1
            } else {
                1
            };
        } else {
            regional_run = 0;
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

'''
if anchor not in text:
    raise SystemExit("unicode break marker not found")
text = text.replace(anchor, helpers + anchor, 1)

text = text.replace(
    '''        if is_mandatory_break(character) {
            opportunities.push(BreakOpportunity {
                index: boundary,
                kind: BreakKind::Mandatory,
            });
            continue;
        }
''',
    '''        if is_mandatory_break(character) {
            if is_grapheme_boundary(text, boundary) {
                opportunities.push(BreakOpportunity {
                    index: boundary,
                    kind: BreakKind::Mandatory,
                });
            }
            continue;
        }
''',
    1,
)

text = text.replace(
    '''        if is_breakable_whitespace(character)
            || character == '-'
            || (is_cjk_ideograph(character) && next.is_some_and(is_cjk_ideograph))
        {
''',
    '''        if is_grapheme_boundary(text, boundary)
            && (is_breakable_whitespace(character)
                || character == '-'
                || (is_cjk_ideograph(character) && next.is_some_and(is_cjk_ideograph)))
        {
''',
    1,
)

text = text.replace(
    '''                    (emergency > line_start && !is_non_breaking_boundary(&run.text, emergency))
                        .then_some(emergency)
''',
    '''                    (emergency > line_start
                        && is_grapheme_boundary(&run.text, emergency)
                        && !is_non_breaking_boundary(&run.text, emergency))
                        .then_some(emergency)
''',
    1,
)

module_end = text.rfind("\n}")
extra = r'''

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
            vec![BreakOpportunity { index: 3, kind: BreakKind::Mandatory }]
        );
    }
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
arch = architecture.read_text()
marker = "Line breaking now consumes explicit Unicode-aware break opportunities."
pos = arch.find(marker)
if pos < 0:
    raise SystemExit("architecture marker not found")
end = arch.find("\n\n", pos)
addition = "\n\nGrapheme safety is enforced before shaping and line breaking: `TextRange` remains scalar-index based, while `GlyphCluster` may cover multiple scalar values. The deterministic R0 classifier keeps combining marks, variation selectors, emoji modifiers, CRLF, regional-indicator pairs, and basic emoji ZWJ sequences indivisible. This is a UAX #29-oriented bootstrap subset rather than full conformance."
arch = arch[:end] + addition + arch[end:]
architecture.write_text(arch)

backlog = Path("docs/R0-BACKLOG.md")
backlog_text = backlog.read_text()
for old, new in [
    ("- [ ] Grapheme-cluster safety", "- [x] Grapheme-cluster safety foundation for shaping and line breaking"),
    ("- [ ] Grapheme cluster safety", "- [x] Grapheme-cluster safety foundation for shaping and line breaking"),
]:
    if old in backlog_text:
        backlog_text = backlog_text.replace(old, new, 1)
backlog.write_text(backlog_text)

Path("docs/adr/0018-grapheme-cluster-safety.md").write_text("""# ADR-0018: Grapheme-cluster safety

## Status

Accepted.

## Context

Unicode-aware line breaking introduced legal and mandatory break opportunities, but the bootstrap shaper still emitted one cluster per scalar value. Emergency wrapping could therefore split combining sequences or emoji sequences internally.

## Decision

Keep `TextRange` indexed by Unicode scalar position, but allow each `GlyphCluster` to span multiple scalar values. Introduce deterministic grapheme-safe boundaries before shaping and require line-break opportunities and emergency breaks to land on those boundaries.

The R0 classifier preserves combining marks, variation selectors, emoji modifiers, CRLF, regional-indicator pairs, and basic emoji ZWJ sequences. It is intentionally a UAX #29-oriented subset, not full Unicode grapheme segmentation conformance.

## Consequences

Shaping, line breaking, fragmentation, and retained paint can now treat clusters as indivisible without changing existing `TextRange` or fragment identity contracts. A standards-complete grapheme segmenter can replace the bootstrap classifier later.
""")
