from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text()

anchor = '''fn is_cjk_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
    )
}
'''
replacement = anchor + '''
fn is_non_breaking_boundary(text: &str, index: usize) -> bool {
    let characters = text.chars().collect::<Vec<_>>();
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index);
    previous
        .into_iter()
        .chain(next)
        .any(|character| matches!(character, '\\u{00a0}' | '\\u{202f}'))
}
'''
if anchor not in text:
    raise SystemExit("CJK helper marker not found")
text = text.replace(anchor, replacement, 1)

start = text.index("impl LineBreaker for UnicodeLineBreaker {")
end = text.index("\n}\n\npub type FixedAdvanceLineBreaker", start) + 2
old = text[start:end]
new = '''impl LineBreaker for UnicodeLineBreaker {
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

            if available_width.is_finite() && available_width >= 0.0 && width > available_width {
                let emergency = cluster.source.start;
                let break_at = last_soft.filter(|value| *value > line_start).or_else(|| {
                    (emergency > line_start && !is_non_breaking_boundary(&run.text, emergency))
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
}'''
text = text[:start] + new + text[end:]
path.write_text(text)
