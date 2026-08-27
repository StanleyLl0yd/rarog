from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()

anchor = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum BreakKind {\n    Soft,\n    Mandatory,\n}\n'''
insert = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum TextDirection {\n    Ltr,\n    Rtl,\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub struct BidiLevel(u8);\n\nimpl BidiLevel {\n    pub const fn new(value: u8) -> Self {\n        Self(value)\n    }\n\n    pub const fn value(self) -> u8 {\n        self.0\n    }\n\n    pub const fn direction(self) -> TextDirection {\n        if self.0 % 2 == 0 {\n            TextDirection::Ltr\n        } else {\n            TextDirection::Rtl\n        }\n    }\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct BidiRun {\n    pub range: TextRange,\n    pub level: BidiLevel,\n}\n\npub fn paragraph_direction(text: &str) -> TextDirection {\n    text.chars()\n        .find_map(strong_direction)\n        .unwrap_or(TextDirection::Ltr)\n}\n\npub fn bidi_runs(text: &str) -> Vec<BidiRun> {\n    let characters = text.chars().collect::<Vec<_>>();\n    if characters.is_empty() {\n        return Vec::new();\n    }\n\n    let base = paragraph_direction(text);\n    let base_level = match base {\n        TextDirection::Ltr => BidiLevel::new(0),\n        TextDirection::Rtl => BidiLevel::new(1),\n    };\n\n    let mut resolved = Vec::with_capacity(characters.len());\n    let mut previous = base;\n    for character in characters.iter().copied() {\n        let direction = strong_direction(character).unwrap_or(previous);\n        resolved.push(direction);\n        if strong_direction(character).is_some() {\n            previous = direction;\n        }\n    }\n\n    let mut runs = Vec::new();\n    let mut start = 0usize;\n    let mut current = resolved[0];\n    for (index, direction) in resolved.iter().copied().enumerate().skip(1) {\n        if direction != current {\n            runs.push(BidiRun {\n                range: TextRange::new(start, index),\n                level: level_for_direction(base_level, current),\n            });\n            start = index;\n            current = direction;\n        }\n    }\n    runs.push(BidiRun {\n        range: TextRange::new(start, characters.len()),\n        level: level_for_direction(base_level, current),\n    });\n    runs\n}\n\npub fn visual_bidi_runs(text: &str) -> Vec<BidiRun> {\n    let mut runs = bidi_runs(text);\n    if runs.is_empty() {\n        return runs;\n    }\n    let max_level = runs.iter().map(|run| run.level.value()).max().unwrap_or(0);\n    let min_odd = runs\n        .iter()\n        .map(|run| run.level.value())\n        .filter(|level| level % 2 == 1)\n        .min();\n    if let Some(min_odd) = min_odd {\n        for level in (min_odd..=max_level).rev() {\n            let mut index = 0usize;\n            while index < runs.len() {\n                if runs[index].level.value() < level {\n                    index += 1;\n                    continue;\n                }\n                let start = index;\n                while index < runs.len() && runs[index].level.value() >= level {\n                    index += 1;\n                }\n                runs[start..index].reverse();\n            }\n        }\n    }\n    runs\n}\n\nfn level_for_direction(base: BidiLevel, direction: TextDirection) -> BidiLevel {\n    match (base.direction(), direction) {\n        (TextDirection::Ltr, TextDirection::Ltr) => BidiLevel::new(0),\n        (TextDirection::Ltr, TextDirection::Rtl) => BidiLevel::new(1),\n        (TextDirection::Rtl, TextDirection::Rtl) => BidiLevel::new(1),\n        (TextDirection::Rtl, TextDirection::Ltr) => BidiLevel::new(2),\n    }\n}\n\nfn strong_direction(character: char) -> Option<TextDirection> {\n    let code = character as u32;\n    if matches!(code, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfefc) {\n        Some(TextDirection::Rtl)\n    } else if character.is_alphabetic() || character.is_ascii_digit() {\n        Some(TextDirection::Ltr)\n    } else {\n        None\n    }\n}\n\n'''
if anchor not in text:
    raise SystemExit('BreakKind anchor not found')
text = text.replace(anchor, insert + anchor, 1)

module_end = text.rfind('\n}')
extra = r'''

    #[test]
    fn bidi_detects_ltr_and_rtl_paragraph_direction() {
        assert_eq!(paragraph_direction("hello"), TextDirection::Ltr);
        assert_eq!(paragraph_direction("שלום"), TextDirection::Rtl);
        assert_eq!(paragraph_direction("مرحبا"), TextDirection::Rtl);
    }

    #[test]
    fn bidi_builds_stable_mixed_direction_runs() {
        let runs = bidi_runs("abc שלום xyz");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0], BidiRun { range: TextRange::new(0, 4), level: BidiLevel::new(0) });
        assert_eq!(runs[1], BidiRun { range: TextRange::new(4, 9), level: BidiLevel::new(1) });
        assert_eq!(runs[2], BidiRun { range: TextRange::new(9, 12), level: BidiLevel::new(0) });
    }

    #[test]
    fn bidi_visual_order_reverses_rtl_run_groups_without_touching_text_ranges() {
        let logical = bidi_runs("אבג abc דהו");
        let visual = visual_bidi_runs("אבג abc דהו");
        assert_eq!(logical.len(), 3);
        assert_eq!(visual.len(), 3);
        assert_eq!(visual[0].range, logical[2].range);
        assert_eq!(visual[1].range, logical[1].range);
        assert_eq!(visual[2].range, logical[0].range);
    }

    #[test]
    fn bidi_keeps_grapheme_ranges_scalar_indexed() {
        let runs = bidi_runs("a e\u{301} שלום");
        assert!(runs.iter().all(|run| run.range.end <= "a e\u{301} שלום".chars().count()));
        assert_eq!(grapheme_boundaries("e\u{301}"), vec![0, 2]);
    }
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

arch = Path('docs/ARCHITECTURE.md')
arch_text = arch.read_text()
append = '''\n\n### Bidirectional text foundation\n\nR0 now exposes explicit `TextDirection`, `BidiLevel`, and `BidiRun` values. Paragraph direction is derived from the first strong character and mixed strong-direction spans are represented as scalar-indexed runs. `visual_bidi_runs()` performs deterministic level-based run reordering while leaving grapheme, shaping, line-breaking, fragment, and retained-paint identities unchanged. This is a UAX #9-oriented bootstrap boundary, not full Unicode Bidirectional Algorithm conformance.\n'''
if '### Bidirectional text foundation' not in arch_text:
    arch.write_text(arch_text.rstrip() + append + '\n')

Path('docs/adr/0019-bidi-foundation.md').write_text('''# ADR-0019: Bidirectional text foundation\n\n## Status\n\nAccepted.\n\n## Context\n\nGrapheme-safe shaping and Unicode-aware line breaking still treated text as a single logical direction. A browser text pipeline needs an explicit boundary between logical source ranges and visual ordering before a real shaping backend can support RTL scripts correctly.\n\n## Decision\n\nIntroduce `TextDirection`, `BidiLevel`, and `BidiRun` in the layout text boundary. Keep all ranges indexed by Unicode scalar position. Determine the bootstrap paragraph direction from the first strong character, group deterministic LTR/RTL runs, and expose a level-based visual run ordering helper.\n\nThe R0 classifier recognizes Hebrew and Arabic-family ranges as RTL and alphabetic/digit text as LTR. Neutral characters inherit the preceding strong direction or paragraph base. This is intentionally a UAX #9-oriented subset rather than full conformance.\n\n## Consequences\n\nLogical source identity stays stable while later shaping and painting stages can consume explicit bidi runs. Full embedding controls, isolates, weak/neutral resolution, mirroring, bracket pairing, and standards-complete level resolution remain future work.\n''')
