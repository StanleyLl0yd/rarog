from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()

anchor = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct BidiRun {\n    pub range: TextRange,\n    pub level: BidiLevel,\n}\n'''
insert = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct ShapingRun {\n    pub range: TextRange,\n    pub face: FontFaceId,\n    pub level: BidiLevel,\n}\n\nimpl ShapingRun {\n    pub const fn direction(self) -> TextDirection {\n        self.level.direction()\n    }\n}\n\npub fn shaping_runs(text: &str, fallback: &FontFallbackChain) -> Vec<ShapingRun> {\n    let fonts = font_runs(text, fallback);\n    shaping_runs_for_font_runs(text, &fonts)\n}\n\nfn shaping_runs_for_font_runs(text: &str, fonts: &[FontRun]) -> Vec<ShapingRun> {\n    let bidi = bidi_runs(text);\n    let mut runs = Vec::new();\n    let mut bidi_index = 0usize;\n    let mut font_index = 0usize;\n\n    while bidi_index < bidi.len() && font_index < fonts.len() {\n        let bidi_run = bidi[bidi_index];\n        let font_run = fonts[font_index];\n        let start = bidi_run.range.start.max(font_run.range.start);\n        let end = bidi_run.range.end.min(font_run.range.end);\n\n        if start < end {\n            debug_assert!(is_grapheme_boundary(text, start));\n            debug_assert!(is_grapheme_boundary(text, end));\n            let run = ShapingRun {\n                range: TextRange::new(start, end),\n                face: font_run.face,\n                level: bidi_run.level,\n            };\n            if let Some(previous) = runs.last_mut() {\n                if previous.face == run.face\n                    && previous.level == run.level\n                    && previous.range.end == run.range.start\n                {\n                    previous.range.end = run.range.end;\n                } else {\n                    runs.push(run);\n                }\n            } else {\n                runs.push(run);\n            }\n        }\n\n        if bidi_run.range.end <= font_run.range.end {\n            bidi_index += 1;\n        }\n        if font_run.range.end <= bidi_run.range.end {\n            font_index += 1;\n        }\n    }\n\n    runs\n}\n\n'''
if anchor not in text:
    raise SystemExit('BidiRun anchor not found')
text = text.replace(anchor, anchor + '\n' + insert, 1)

method_anchor = '''    pub fn character_count(&self) -> usize {\n        self.text.chars().count()\n    }\n'''
method = '''    pub fn shaping_runs(&self) -> Vec<ShapingRun> {\n        shaping_runs_for_font_runs(&self.text, &self.font_runs)\n    }\n\n'''
if method_anchor not in text:
    raise SystemExit('TextRun method anchor not found')
text = text.replace(method_anchor, method + method_anchor, 1)

module_end = text.rfind('\n}')
extra = r'''

    #[test]
    fn shaping_runs_intersect_bidi_and_font_boundaries() {
        let chain = FontFallbackChain::default();
        let runs = shaping_runs("abc שלום 世界", &chain);
        assert_eq!(
            runs,
            vec![
                ShapingRun {
                    range: TextRange::new(0, 4),
                    face: FontFaceId::new(0),
                    level: BidiLevel::new(0),
                },
                ShapingRun {
                    range: TextRange::new(4, 9),
                    face: FontFaceId::new(1),
                    level: BidiLevel::new(1),
                },
                ShapingRun {
                    range: TextRange::new(9, 11),
                    face: FontFaceId::new(2),
                    level: BidiLevel::new(0),
                },
            ]
        );
    }

    #[test]
    fn shaping_runs_split_bidi_even_when_font_face_is_shared() {
        let metrics = FontMetrics {
            ascent: 14.0,
            descent: 4.0,
            line_gap: 0.0,
        };
        let chain = FontFallbackChain {
            faces: vec![FontFace {
                id: FontFaceId::new(9),
                family: FontFamily("Shared LastResort".into()),
                coverage: FontCoverage::LastResort,
                metrics,
                advance: 8.0,
            }],
        };
        let runs = shaping_runs("abc שלום", &chain);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].face, FontFaceId::new(9));
        assert_eq!(runs[1].face, FontFaceId::new(9));
        assert_eq!(runs[0].level, BidiLevel::new(0));
        assert_eq!(runs[1].level, BidiLevel::new(1));
    }

    #[test]
    fn shaping_runs_split_font_fallback_inside_one_bidi_direction() {
        let runs = shaping_runs("abc 世界", &FontFallbackChain::default());
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].level, BidiLevel::new(0));
        assert_eq!(runs[1].level, BidiLevel::new(0));
        assert_eq!(runs[0].face, FontFaceId::new(0));
        assert_eq!(runs[1].face, FontFaceId::new(2));
    }

    #[test]
    fn shaping_runs_preserve_grapheme_safe_source_ranges() {
        let text = "a👩🏽\u{200d}💻אב";
        let runs = shaping_runs(text, &FontFallbackChain::default());
        assert_eq!(grapheme_boundaries(text), vec![0, 1, 5, 6, 7]);
        assert!(runs.iter().all(|run| {
            is_grapheme_boundary(text, run.range.start)
                && is_grapheme_boundary(text, run.range.end)
        }));
        assert_eq!(runs.first().unwrap().range.start, 0);
        assert_eq!(runs.last().unwrap().range.end, text.chars().count());
        assert!(runs.windows(2).all(|pair| pair[0].range.end == pair[1].range.start));
    }

    #[test]
    fn text_run_exposes_stable_shaping_segments() {
        let run = TextRun::new("abc שלום 世界".into());
        let segments = run.shaping_runs();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].direction(), TextDirection::Ltr);
        assert_eq!(segments[1].direction(), TextDirection::Rtl);
        assert_eq!(segments[2].direction(), TextDirection::Ltr);
    }
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

arch = Path('docs/ARCHITECTURE.md')
arch_text = arch.read_text()
section = '''\n\n### Shaping segmentation foundation\n\nR0 now derives scalar-indexed `ShapingRun` segments by intersecting logical bidi runs with grapheme-safe font fallback runs. Every shaping segment has exactly one source range, one `FontFaceId`, and one `BidiLevel`/direction, and adjacent segments with identical shaping state are coalesced. This is the handoff boundary for a future OpenType shaper: a real backend can shape each segment independently without changing DOM, source ranges, line breaking, fragment identity, or retained paint.\n'''
if '### Shaping segmentation foundation' not in arch_text:
    arch.write_text(arch_text.rstrip() + section + '\n')

backlog = Path('docs/R0-BACKLOG.md')
if backlog.exists():
    value = backlog.read_text()
    for candidate in [
        '- [ ] Shaping segmentation',
        '- [ ] Shaping-run segmentation',
        '- [ ] Shaping run segmentation',
    ]:
        if candidate in value:
            value = value.replace(candidate, '- [x] Shaping segmentation foundation', 1)
            break
    backlog.write_text(value)

Path('docs/adr/0021-shaping-segmentation.md').write_text('''# ADR-0021: Shaping segmentation\n\n## Status\n\nAccepted.\n\n## Context\n\nR0 already has grapheme-safe source ranges, logical bidi runs, and deterministic font fallback runs. A real text shaper must receive text with one direction/embedding level and one selected font face; passing a whole mixed-direction or mixed-font `TextRun` would force shaping backends to duplicate segmentation policy.\n\n## Decision\n\nIntroduce scalar-indexed `ShapingRun` values containing `TextRange`, `FontFaceId`, and `BidiLevel`. Build them as the ordered intersection of logical bidi runs and font fallback runs, preserve grapheme-safe boundaries, and coalesce adjacent intersections when face and level are identical. `TextRun` exposes the resulting shaping segments directly.\n\n## Consequences\n\nThe future OpenType backend can shape one segment at a time and return glyph clusters without owning bidi or fallback policy. Script/language tags, OpenType feature selection, variation axes, vertical text, full UAX #9 resolution, and platform font discovery remain separate future layers.\n''')
