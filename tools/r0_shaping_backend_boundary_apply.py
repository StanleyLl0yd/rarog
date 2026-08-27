from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()

anchor = '''#[derive(Clone, Copy, Debug, PartialEq)]\npub struct GlyphCluster {\n    pub source: TextRange,\n    pub advance: f32,\n}\n\n#[derive(Clone, Debug, PartialEq)]\npub struct ShapedText {\n    pub clusters: Vec<GlyphCluster>,\n    pub advance: f32,\n    pub metrics: FontMetrics,\n}\n'''
insert = '''#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct GlyphId(u32);\n\nimpl GlyphId {\n    pub const fn new(value: u32) -> Self {\n        Self(value)\n    }\n\n    pub const fn value(self) -> u32 {\n        self.0\n    }\n}\n\n#[derive(Clone, Copy, Debug, Default, PartialEq)]\npub struct GlyphOffset {\n    pub x: f32,\n    pub y: f32,\n}\n\n#[derive(Clone, Copy, Debug, PartialEq)]\npub struct PositionedGlyph {\n    pub id: GlyphId,\n    pub source: TextRange,\n    pub advance: f32,\n    pub offset: GlyphOffset,\n}\n\n#[derive(Clone, Debug, PartialEq)]\npub struct ShapedRun {\n    pub run: ShapingRun,\n    pub glyphs: Vec<PositionedGlyph>,\n    pub advance: f32,\n    pub metrics: FontMetrics,\n}\n\npub trait ShapingBackend {\n    fn shape_run(&self, text: &str, run: ShapingRun, face: &FontFace) -> ShapedRun;\n}\n\n'''
if anchor not in text:
    raise SystemExit('GlyphCluster anchor not found')
text = text.replace(anchor, anchor + '\n' + insert, 1)

impl_anchor = '''impl TextShaper for FixedTextShaper {\n    fn shape(&self, text: &str) -> ShapedText {\n'''
backend_impl = '''impl ShapingBackend for FixedTextShaper {\n    fn shape_run(&self, text: &str, run: ShapingRun, face: &FontFace) -> ShapedRun {\n        let characters = text.chars().collect::<Vec<_>>();\n        let boundaries = grapheme_boundaries(text);\n        let mut glyphs = boundaries\n            .windows(2)\n            .filter_map(|window| {\n                let start = window[0];\n                let end = window[1];\n                if start < run.range.start || end > run.range.end {\n                    return None;\n                }\n                let slice = &characters[start..end];\n                let mandatory = slice.iter().copied().any(is_mandatory_break);\n                let glyph_id = slice\n                    .first()\n                    .copied()\n                    .map(|character| GlyphId::new(character as u32))\n                    .unwrap_or_else(|| GlyphId::new(0));\n                Some(PositionedGlyph {\n                    id: glyph_id,\n                    source: TextRange::new(start, end),\n                    advance: if mandatory { 0.0 } else { face.advance },\n                    offset: GlyphOffset::default(),\n                })\n            })\n            .collect::<Vec<_>>();\n        if run.direction() == TextDirection::Rtl {\n            glyphs.reverse();\n        }\n        ShapedRun {\n            advance: glyphs.iter().map(|glyph| glyph.advance).sum(),\n            glyphs,\n            run,\n            metrics: face.metrics,\n        }\n    }\n}\n\n'''
if impl_anchor not in text:
    raise SystemExit('FixedTextShaper impl anchor not found')
text = text.replace(impl_anchor, backend_impl + impl_anchor, 1)

method_anchor = '''    pub fn shaping_runs(&self) -> Vec<ShapingRun> {\n        shaping_runs_for_font_runs(&self.text, &self.font_runs)\n    }\n'''
method = '''    pub fn shape_with_backend<B: ShapingBackend>(\n        &self,\n        fallback: &FontFallbackChain,\n        backend: &B,\n    ) -> Vec<ShapedRun> {\n        self.shaping_runs()\n            .into_iter()\n            .filter_map(|run| {\n                fallback\n                    .face(run.face)\n                    .map(|face| backend.shape_run(&self.text, run, face))\n            })\n            .collect()\n    }\n\n'''
if method_anchor not in text:
    raise SystemExit('TextRun shaping_runs anchor not found')
text = text.replace(method_anchor, method_anchor + '\n' + method, 1)

module_end = text.rfind('\n}')
extra = r'''

    #[test]
    fn shaping_backend_returns_glyph_ids_advances_offsets_and_source_mapping() {
        let text = "a👩🏽\u{200d}💻b";
        let fallback = FontFallbackChain::default();
        let runs = shaping_runs(text, &fallback);
        let backend = FixedTextShaper::default();
        let shaped = runs
            .iter()
            .map(|run| backend.shape_run(text, *run, fallback.face(run.face).unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(shaped.len(), 3);
        assert_eq!(shaped[0].glyphs[0].id, GlyphId::new('a' as u32));
        assert_eq!(shaped[0].glyphs[0].source, TextRange::new(0, 1));
        assert_eq!(shaped[1].glyphs.len(), 1);
        assert_eq!(shaped[1].glyphs[0].source, TextRange::new(1, 5));
        assert_eq!(shaped[1].glyphs[0].offset, GlyphOffset::default());
        assert_eq!(shaped[2].glyphs[0].source, TextRange::new(5, 6));
        assert_eq!(shaped.iter().map(|run| run.advance).sum::<f32>(), 24.0);
    }

    #[test]
    fn shaping_backend_uses_selected_face_metrics_and_advance() {
        let metrics = FontMetrics {
            ascent: 10.0,
            descent: 3.0,
            line_gap: 2.0,
        };
        let face = FontFace {
            id: FontFaceId::new(42),
            family: FontFamily("Custom".into()),
            coverage: FontCoverage::LastResort,
            metrics,
            advance: 11.0,
        };
        let run = ShapingRun {
            range: TextRange::new(0, 2),
            face: face.id,
            level: BidiLevel::new(0),
        };
        let shaped = FixedTextShaper::default().shape_run("ab", run, &face);
        assert_eq!(shaped.metrics, metrics);
        assert_eq!(shaped.advance, 22.0);
        assert_eq!(shaped.glyphs.iter().map(|glyph| glyph.advance).collect::<Vec<_>>(), vec![11.0, 11.0]);
    }

    #[test]
    fn rtl_shaping_run_returns_visual_glyph_order_with_logical_source_mapping() {
        let text = "אב";
        let fallback = FontFallbackChain::default();
        let run = shaping_runs(text, &fallback)[0];
        assert_eq!(run.direction(), TextDirection::Rtl);
        let shaped = FixedTextShaper::default().shape_run(text, run, fallback.face(run.face).unwrap());
        assert_eq!(shaped.glyphs.len(), 2);
        assert_eq!(shaped.glyphs[0].source, TextRange::new(1, 2));
        assert_eq!(shaped.glyphs[1].source, TextRange::new(0, 1));
    }

    #[test]
    fn text_run_can_shape_all_segments_through_backend_boundary() {
        let fallback = FontFallbackChain::default();
        let run = TextRun::with_fallback("abc שלום 世界".into(), &fallback);
        let shaped = run.shape_with_backend(&fallback, &FixedTextShaper::default());
        assert_eq!(shaped.len(), 3);
        assert_eq!(shaped[0].run.face, FontFaceId::new(0));
        assert_eq!(shaped[1].run.face, FontFaceId::new(1));
        assert_eq!(shaped[2].run.face, FontFaceId::new(2));
        assert_eq!(shaped[1].run.direction(), TextDirection::Rtl);
        assert!(shaped.iter().all(|segment| !segment.glyphs.is_empty()));
    }
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

arch = Path('docs/ARCHITECTURE.md')
arch_text = arch.read_text()
section = '''\n\n### Shaping backend boundary\n\nR0 now separates shaping segmentation from shaping execution. `ShapingBackend` receives one `ShapingRun` plus its selected `FontFace` and returns a `ShapedRun` containing positioned glyph IDs, per-glyph advances/offsets, and scalar-indexed source-cluster mapping. The deterministic `FixedTextShaper` implements this contract as the bootstrap backend while the existing aggregate `ShapedText` remains the line-breaking input. A future OpenType backend can therefore replace glyph generation without owning bidi, font fallback, source identity, fragmentation, or retained-paint policy.\n'''
if '### Shaping backend boundary' not in arch_text:
    arch.write_text(arch_text.rstrip() + section + '\n')

Path('docs/adr/0022-shaping-backend-boundary.md').write_text('''# ADR-0022: Shaping backend boundary\n\n## Status\n\nAccepted.\n\n## Context\n\nR0 already segments text into grapheme-safe shaping runs with one font face and one bidi level. The remaining boundary must support a real OpenType shaper without leaking backend-specific glyph data into bidi, fallback, line breaking, fragments, or paint.\n\n## Decision\n\nIntroduce `ShapingBackend::shape_run`, taking source text, one `ShapingRun`, and its resolved `FontFace`. The backend returns a `ShapedRun` containing glyph IDs, per-glyph advances and offsets, source-cluster ranges, aggregate advance, and font metrics. Keep the current aggregate `ShapedText` contract for line breaking until the line-layout layer is ready to consume backend glyph runs directly.\n\nThe bootstrap `FixedTextShaper` implements both contracts. Its backend implementation emits one deterministic glyph per grapheme cluster, uses the selected face metrics/advance, preserves logical source ranges, and reverses glyph order for RTL shaping runs while retaining source mapping.\n\n## Consequences\n\nA real OpenType shaping implementation can be introduced behind the new trait without changing bidi segmentation, font fallback, source identity, fragmentation, or retained paint. Script/language tags, OpenType feature selection, variation axes, glyph extents, vertical text, and platform font discovery remain future work.\n''')
