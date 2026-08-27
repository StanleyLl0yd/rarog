from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()

anchor = '''pub trait TextShaper {\n    fn shape(&self, text: &str) -> ShapedText;\n}\n'''
insert = '''#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct FontFaceId(u16);\n\nimpl FontFaceId {\n    pub const fn new(value: u16) -> Self {\n        Self(value)\n    }\n\n    pub const fn value(self) -> u16 {\n        self.0\n    }\n}\n\n#[derive(Clone, Debug, PartialEq, Eq)]\npub struct FontFamily(pub String);\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum FontCoverage {\n    LatinCyrillic,\n    HebrewArabic,\n    Cjk,\n    Emoji,\n    LastResort,\n}\n\n#[derive(Clone, Debug, PartialEq)]\npub struct FontFace {\n    pub id: FontFaceId,\n    pub family: FontFamily,\n    pub coverage: FontCoverage,\n    pub metrics: FontMetrics,\n    pub advance: f32,\n}\n\nimpl FontFace {\n    pub fn covers(&self, character: char) -> bool {\n        let code = character as u32;\n        if is_grapheme_extend(character)\n            || character.is_whitespace()\n            || character.is_ascii_punctuation()\n            || matches!(code, 0x2000..=0x206f)\n        {\n            return true;\n        }\n        match self.coverage {\n            FontCoverage::LatinCyrillic => {\n                character.is_ascii_alphanumeric()\n                    || matches!(code, 0x00a0..=0x024f | 0x0370..=0x052f)\n            }\n            FontCoverage::HebrewArabic => {\n                matches!(code, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfefc)\n            }\n            FontCoverage::Cjk => {\n                matches!(code, 0x2e80..=0x9fff | 0xf900..=0xfaff | 0x3040..=0x30ff | 0xac00..=0xd7af)\n            }\n            FontCoverage::Emoji => {\n                is_extended_pictographic(character) || is_regional_indicator(character)\n            }\n            FontCoverage::LastResort => true,\n        }\n    }\n}\n\n#[derive(Clone, Debug, PartialEq)]\npub struct FontFallbackChain {\n    pub faces: Vec<FontFace>,\n}\n\nimpl Default for FontFallbackChain {\n    fn default() -> Self {\n        let metrics = FontMetrics {\n            ascent: 14.0,\n            descent: 4.0,\n            line_gap: 0.0,\n        };\n        Self {\n            faces: vec![\n                FontFace {\n                    id: FontFaceId::new(0),\n                    family: FontFamily("Rarog Sans".into()),\n                    coverage: FontCoverage::LatinCyrillic,\n                    metrics,\n                    advance: 8.0,\n                },\n                FontFace {\n                    id: FontFaceId::new(1),\n                    family: FontFamily("Rarog RTL".into()),\n                    coverage: FontCoverage::HebrewArabic,\n                    metrics,\n                    advance: 8.0,\n                },\n                FontFace {\n                    id: FontFaceId::new(2),\n                    family: FontFamily("Rarog CJK".into()),\n                    coverage: FontCoverage::Cjk,\n                    metrics,\n                    advance: 8.0,\n                },\n                FontFace {\n                    id: FontFaceId::new(3),\n                    family: FontFamily("Rarog Emoji".into()),\n                    coverage: FontCoverage::Emoji,\n                    metrics,\n                    advance: 8.0,\n                },\n                FontFace {\n                    id: FontFaceId::new(4),\n                    family: FontFamily("Rarog LastResort".into()),\n                    coverage: FontCoverage::LastResort,\n                    metrics,\n                    advance: 8.0,\n                },\n            ],\n        }\n    }\n}\n\nimpl FontFallbackChain {\n    pub fn face(&self, id: FontFaceId) -> Option<&FontFace> {\n        self.faces.iter().find(|face| face.id == id)\n    }\n\n    pub fn select_face_for_range(&self, text: &str, range: TextRange) -> Option<FontFaceId> {\n        let characters = text.chars().collect::<Vec<_>>();\n        let slice = characters.get(range.start..range.end)?;\n        self.faces\n            .iter()\n            .find(|face| slice.iter().copied().all(|character| face.covers(character)))\n            .map(|face| face.id)\n    }\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct FontRun {\n    pub range: TextRange,\n    pub face: FontFaceId,\n}\n\npub fn font_runs(text: &str, chain: &FontFallbackChain) -> Vec<FontRun> {\n    let boundaries = grapheme_boundaries(text);\n    if boundaries.len() < 2 {\n        return Vec::new();\n    }\n\n    let mut runs = Vec::new();\n    for window in boundaries.windows(2) {\n        let range = TextRange::new(window[0], window[1]);\n        let face = chain\n            .select_face_for_range(text, range)\n            .or_else(|| chain.faces.last().map(|face| face.id))\n            .expect("font fallback chain must contain at least one face");\n        if let Some(previous) = runs.last_mut() {\n            if previous.face == face && previous.range.end == range.start {\n                previous.range.end = range.end;\n                continue;\n            }\n        }\n        runs.push(FontRun { range, face });\n    }\n    runs\n}\n\n'''
if anchor not in text:
    raise SystemExit('TextShaper anchor not found')
text = text.replace(anchor, insert + anchor, 1)

old = '''pub struct TextRun {\n    pub text: String,\n    pub shaped: ShapedText,\n    pub advance: f32,\n    pub line_height: f32,\n}\n'''
new = '''pub struct TextRun {\n    pub text: String,\n    pub shaped: ShapedText,\n    pub font_runs: Vec<FontRun>,\n    pub advance: f32,\n    pub line_height: f32,\n}\n'''
if old not in text:
    raise SystemExit('TextRun struct marker not found')
text = text.replace(old, new, 1)

old = '''    pub fn new(text: String) -> Self {\n        let shaper = FixedTextShaper::default();\n        let shaped = shaper.shape(&text);\n        Self {\n            text,\n            advance: shaped.advance,\n            line_height: shaped.metrics.line_height(),\n            shaped,\n        }\n    }\n'''
new = '''    pub fn new(text: String) -> Self {\n        Self::with_fallback(text, &FontFallbackChain::default())\n    }\n\n    pub fn with_fallback(text: String, fallback: &FontFallbackChain) -> Self {\n        let shaper = FixedTextShaper::default();\n        let shaped = shaper.shape(&text);\n        let font_runs = font_runs(&text, fallback);\n        Self {\n            text,\n            advance: shaped.advance,\n            line_height: shaped.metrics.line_height(),\n            shaped,\n            font_runs,\n        }\n    }\n'''
if old not in text:
    raise SystemExit('TextRun constructor marker not found')
text = text.replace(old, new, 1)

module_end = text.rfind('\n}')
extra = r'''

    #[test]
    fn font_fallback_keeps_latin_and_cyrillic_in_primary_face() {
        let chain = FontFallbackChain::default();
        let runs = font_runs("Hello Привет", &chain);
        assert_eq!(runs, vec![FontRun { range: TextRange::new(0, 12), face: FontFaceId::new(0) }]);
    }

    #[test]
    fn font_fallback_splits_mixed_scripts_into_stable_runs() {
        let chain = FontFallbackChain::default();
        let runs = font_runs("abc שלום 世界", &chain);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].face, FontFaceId::new(0));
        assert_eq!(runs[1].face, FontFaceId::new(1));
        assert_eq!(runs[2].face, FontFaceId::new(2));
        assert_eq!(runs[0].range, TextRange::new(0, 4));
        assert_eq!(runs[1].range, TextRange::new(4, 9));
        assert_eq!(runs[2].range, TextRange::new(9, 11));
    }

    #[test]
    fn font_fallback_never_splits_grapheme_clusters() {
        let chain = FontFallbackChain::default();
        let text = "a👩🏽\u{200d}💻b";
        let runs = font_runs(text, &chain);
        assert_eq!(grapheme_boundaries(text), vec![0, 1, 5, 6]);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[1], FontRun { range: TextRange::new(1, 5), face: FontFaceId::new(3) });
    }

    #[test]
    fn font_fallback_has_deterministic_last_resort() {
        let chain = FontFallbackChain::default();
        let runs = font_runs("\u{10300}", &chain);
        assert_eq!(runs, vec![FontRun { range: TextRange::new(0, 1), face: FontFaceId::new(4) }]);
        assert_eq!(chain.face(FontFaceId::new(4)).unwrap().family.0, "Rarog LastResort");
    }

    #[test]
    fn text_run_exposes_font_runs_without_changing_source_ranges() {
        let run = TextRun::new("abc שלום".into());
        assert_eq!(run.font_runs.len(), 2);
        assert_eq!(run.font_runs[0].range, TextRange::new(0, 4));
        assert_eq!(run.font_runs[1].range, TextRange::new(4, 8));
        assert_eq!(run.character_count(), 8);
    }
'''
text = text[:module_end] + extra + text[module_end:]
path.write_text(text)

arch = Path('docs/ARCHITECTURE.md')
arch_text = arch.read_text()
append = '''\n\n### Font fallback foundation\n\nR0 now models font selection explicitly through `FontFaceId`, `FontFamily`, `FontFace`, `FontFallbackChain`, and scalar-indexed `FontRun` values. Fallback selection occurs only on grapheme-cluster boundaries, so combining sequences and emoji ZWJ clusters cannot be split between faces. The deterministic bootstrap chain covers Latin/Cyrillic, Hebrew/Arabic, CJK, emoji, and a mandatory LastResort face. These are architectural coverage classes rather than bundled fonts; a platform font database and real shaping backend can replace the selector without changing source, bidi, fragment, or retained-paint identities.\n'''
if '### Font fallback foundation' not in arch_text:
    arch.write_text(arch_text.rstrip() + append + '\n')

backlog = Path('docs/R0-BACKLOG.md')
if backlog.exists():
    value = backlog.read_text()
    candidates = [
        '- [ ] Font fallback model',
        '- [ ] Font fallback foundation',
        '- [ ] Font fallback',
    ]
    for candidate in candidates:
        if candidate in value:
            value = value.replace(candidate, '- [x] Font fallback foundation', 1)
            break
    backlog.write_text(value)

Path('docs/adr/0020-font-fallback-foundation.md').write_text('''# ADR-0020: Font fallback foundation\n\n## Status\n\nAccepted.\n\n## Context\n\nThe text pipeline has grapheme-safe segmentation and explicit bidi runs, but shaping still assumes one synthetic font for every character. A real browser must select different font faces for unsupported scripts without changing logical source identity or splitting grapheme clusters.\n\n## Decision\n\nIntroduce explicit font-face identity, families, coverage classes, a deterministic fallback chain, and scalar-indexed `FontRun` values. Select fallback per grapheme cluster and coalesce adjacent clusters using the same face. The bootstrap chain contains Latin/Cyrillic, Hebrew/Arabic, CJK, emoji, and mandatory LastResort faces.\n\n`TextRun` exposes the selected font runs while the existing bootstrap shaper remains unchanged. A later shaping backend may shape each `(bidi run × font run)` segment independently.\n\n## Consequences\n\nFont fallback can evolve independently of DOM, bidi, line breaking, fragmentation, and retained paint. The R0 faces are deterministic architecture placeholders, not real font files or platform font discovery. Script/language-sensitive fallback, variable fonts, OpenType features, platform enumeration, and glyph-level fallback remain future work.\n''')
