from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if old not in text:
        raise SystemExit(f"missing patch marker in {path}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


cargo = Path("crates/rarog-layout/Cargo.toml")
text = cargo.read_text()
text = text.replace(
    'rarog-types = { path = "../rarog-types" }\n',
    'rarog-types = { path = "../rarog-types" }\nharfrust = "=0.13.3"\n',
    1,
)
text += '\n[dev-dependencies]\nfont-test-data = "=0.7.0"\n'
cargo.write_text(text)

layout = Path("crates/rarog-layout/src/lib.rs")
text = layout.read_text()
text = text.replace(
    'use rarog_types::{Point, Rect, Size};\n',
    'use rarog_types::{Point, Rect, Size};\nuse std::collections::{BTreeMap, BTreeSet};\nuse std::fmt;\nuse std::sync::Arc;\n',
    1,
)

old_trait = '''pub trait ShapingBackend {\n    fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun;\n}\n'''
new_trait = r'''#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapingError {
    MissingFontFace(FontFaceId),
    MissingOpenTypeData(FontFaceId),
    InvalidOpenTypeData(FontFaceId),
    InvalidLanguage(String),
    InvalidRunRange(TextRange),
    ClusterIndexOverflow(usize),
    InvalidCluster { cluster: usize, range: TextRange },
    InvalidPixelsPerEm,
}

impl fmt::Display for ShapingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFontFace(face) => write!(formatter, "missing font face {}", face.value()),
            Self::MissingOpenTypeData(face) => {
                write!(formatter, "font face {} has no OpenType data", face.value())
            }
            Self::InvalidOpenTypeData(face) => {
                write!(formatter, "font face {} contains invalid OpenType data", face.value())
            }
            Self::InvalidLanguage(language) => write!(formatter, "invalid shaping language {language:?}"),
            Self::InvalidRunRange(range) => write!(
                formatter,
                "invalid shaping run range {}..{}",
                range.start, range.end
            ),
            Self::ClusterIndexOverflow(index) => {
                write!(formatter, "shaping cluster index {index} exceeds u32")
            }
            Self::InvalidCluster { cluster, range } => write!(
                formatter,
                "shaper returned cluster {cluster} outside {}..{}",
                range.start, range.end
            ),
            Self::InvalidPixelsPerEm => formatter.write_str("pixels-per-em must be finite and positive"),
        }
    }
}

impl std::error::Error for ShapingError {}

pub trait ShapingBackend {
    fn shape_run(
        &self,
        text: &str,
        request: &ShapingRequest,
        face: &FontFace,
    ) -> Result<ShapedRun, ShapingError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenTypeFontData {
    bytes: Arc<[u8]>,
    face_index: u32,
    pixels_per_em: f32,
}

impl OpenTypeFontData {
    pub fn try_new(
        bytes: impl Into<Arc<[u8]>>,
        face_index: u32,
        pixels_per_em: f32,
    ) -> Result<Self, ShapingError> {
        if !pixels_per_em.is_finite() || pixels_per_em <= 0.0 {
            return Err(ShapingError::InvalidPixelsPerEm);
        }
        let bytes = bytes.into();
        harfrust::FontRef::from_index(&bytes, face_index)
            .map_err(|_| ShapingError::InvalidOpenTypeData(FontFaceId::new(0)))?;
        Ok(Self {
            bytes,
            face_index,
            pixels_per_em,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    pub const fn pixels_per_em(&self) -> f32 {
        self.pixels_per_em
    }
}

#[derive(Clone, Debug, Default)]
pub struct OpenTypeShaper {
    fonts: BTreeMap<FontFaceId, OpenTypeFontData>,
}

impl OpenTypeShaper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_font(&mut self, face: FontFaceId, data: OpenTypeFontData) -> Option<OpenTypeFontData> {
        self.fonts.insert(face, data)
    }

    pub fn remove_font(&mut self, face: FontFaceId) -> Option<OpenTypeFontData> {
        self.fonts.remove(&face)
    }

    pub fn contains_font(&self, face: FontFaceId) -> bool {
        self.fonts.contains_key(&face)
    }
}
'''
if old_trait not in text:
    raise SystemExit("missing shaping trait marker")
text = text.replace(old_trait, new_trait, 1)

fixed_start = text.index('impl ShapingBackend for FixedTextShaper {')
fixed_end = text.index('\n}\n\nimpl TextShaper for FixedTextShaper', fixed_start) + 2
fixed_old = text[fixed_start:fixed_end]
fixed_new = fixed_old.replace(
    'fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun {',
    'fn shape_run(\n        &self,\n        text: &str,\n        request: &ShapingRequest,\n        face: &FontFace,\n    ) -> Result<ShapedRun, ShapingError> {',
    1,
)
fixed_new = fixed_new.replace(
    '        ShapedRun {\n            advance: glyphs.iter().map(|glyph| glyph.advance).sum(),\n            glyphs,\n            run,\n            metrics: face.metrics,\n        }\n',
    '        Ok(ShapedRun {\n            advance: glyphs.iter().map(|glyph| glyph.advance).sum(),\n            glyphs,\n            run,\n            metrics: face.metrics,\n        })\n',
    1,
)
text = text[:fixed_start] + fixed_new + text[fixed_end:]

opentype_impl = r'''

impl ShapingBackend for OpenTypeShaper {
    fn shape_run(
        &self,
        text: &str,
        request: &ShapingRequest,
        face: &FontFace,
    ) -> Result<ShapedRun, ShapingError> {
        const POSITION_SCALE: f32 = 64.0;

        let data = self
            .fonts
            .get(&face.id)
            .ok_or(ShapingError::MissingOpenTypeData(face.id))?;
        let font = harfrust::FontRef::from_index(data.bytes(), data.face_index())
            .map_err(|_| ShapingError::InvalidOpenTypeData(face.id))?;
        let characters = text.chars().collect::<Vec<_>>();
        let range = request.run.range;
        let slice = characters
            .get(range.start..range.end)
            .ok_or(ShapingError::InvalidRunRange(range))?;

        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.reserve(slice.len());
        for (offset, character) in slice.iter().copied().enumerate() {
            let cluster = range.start.saturating_add(offset);
            let cluster = u32::try_from(cluster)
                .map_err(|_| ShapingError::ClusterIndexOverflow(cluster))?;
            buffer.add(character, cluster);
        }
        buffer.set_direction(match request.run.direction() {
            TextDirection::Ltr => harfrust::Direction::LeftToRight,
            TextDirection::Rtl => harfrust::Direction::RightToLeft,
        });
        buffer.set_script(harfrust_script(request.script));
        let language = request
            .language
            .as_str()
            .parse::<harfrust::Language>()
            .map_err(|_| ShapingError::InvalidLanguage(request.language.as_str().into()))?;
        buffer.set_language(language);

        let features = request
            .features
            .iter()
            .map(|feature| {
                harfrust::Feature::new(
                    harfrust::Tag::from_u32(feature.tag.value()),
                    feature.value,
                    ..,
                )
            })
            .collect::<Vec<_>>();
        let variations = request
            .variations
            .iter()
            .map(|coordinate| harfrust::Variation {
                tag: harfrust::Tag::from_u32(coordinate.axis.value()),
                value: coordinate.value,
            })
            .collect::<Vec<_>>();
        let instance = harfrust::ShaperInstance::from_variations(&font, variations);
        let data_cache = harfrust::ShaperData::new(&font);
        let shaper = data_cache.shaper(&font).instance(Some(&instance)).build();
        let scaled = data.pixels_per_em() * POSITION_SCALE;
        if !scaled.is_finite() || scaled <= 0.0 || scaled > i32::MAX as f32 {
            return Err(ShapingError::InvalidPixelsPerEm);
        }
        let output = shaper.shape(
            buffer,
            harfrust::ShapeOptions::new()
                .features(&features)
                .scale(Some(scaled.round() as i32)),
        );
        let infos = output.glyph_infos();
        let positions = output.glyph_positions();
        let mut cluster_starts = infos
            .iter()
            .map(|info| info.cluster as usize)
            .collect::<BTreeSet<_>>();
        cluster_starts.insert(range.end);
        let cluster_starts = cluster_starts.into_iter().collect::<Vec<_>>();
        for cluster in cluster_starts.iter().copied().filter(|cluster| *cluster != range.end) {
            if cluster < range.start || cluster >= range.end {
                return Err(ShapingError::InvalidCluster { cluster, range });
            }
        }

        let mut glyphs = Vec::with_capacity(infos.len());
        for (info, position) in infos.iter().zip(positions.iter()) {
            let cluster = info.cluster as usize;
            let next = cluster_starts
                .iter()
                .copied()
                .find(|candidate| *candidate > cluster)
                .ok_or(ShapingError::InvalidCluster { cluster, range })?;
            glyphs.push(PositionedGlyph {
                id: GlyphId::new(info.glyph_id),
                source: TextRange::new(cluster, next),
                advance: position.x_advance as f32 / POSITION_SCALE,
                offset: GlyphOffset {
                    x: position.x_offset as f32 / POSITION_SCALE,
                    y: position.y_offset as f32 / POSITION_SCALE,
                },
            });
        }
        Ok(ShapedRun {
            advance: glyphs.iter().map(|glyph| glyph.advance).sum(),
            glyphs,
            run: request.run,
            metrics: face.metrics,
        })
    }
}

fn harfrust_script(script: ShapingScript) -> harfrust::Script {
    match script {
        ShapingScript::Common | ShapingScript::Emoji => harfrust::script::COMMON,
        ShapingScript::Latin => harfrust::script::LATIN,
        ShapingScript::Cyrillic => harfrust::script::CYRILLIC,
        ShapingScript::Hebrew => harfrust::script::HEBREW,
        ShapingScript::Arabic => harfrust::script::ARABIC,
        ShapingScript::Han => harfrust::script::HAN,
        ShapingScript::Unknown => harfrust::script::UNKNOWN,
    }
}
'''
anchor = '\nimpl TextShaper for FixedTextShaper {'
text = text.replace(anchor, opentype_impl + anchor, 1)

old_shape = '''    pub fn shape_with_backend<B: ShapingBackend>(\n        &self,\n        fallback: &FontFallbackChain,\n        backend: &B,\n    ) -> Vec<ShapedRun> {\n        self.shaping_requests()\n            .into_iter()\n            .filter_map(|request| {\n                fallback\n                    .face(request.run.face)\n                    .map(|face| backend.shape_run(&self.text, &request, face))\n            })\n            .collect()\n    }\n'''
new_shape = '''    pub fn shape_with_backend<B: ShapingBackend>(\n        &self,\n        fallback: &FontFallbackChain,\n        backend: &B,\n    ) -> Result<Vec<ShapedRun>, ShapingError> {\n        self.shaping_requests()\n            .into_iter()\n            .map(|request| {\n                let face = fallback\n                    .face(request.run.face)\n                    .ok_or(ShapingError::MissingFontFace(request.run.face))?;\n                backend.shape_run(&self.text, &request, face)\n            })\n            .collect()\n    }\n'''
if old_shape not in text:
    raise SystemExit("missing shape_with_backend marker")
text = text.replace(old_shape, new_shape, 1)

# Existing fixed-backend tests now unwrap explicit shaping results.
text = text.replace('backend.shape_run(\n', 'backend.shape_run(\n')
text = text.replace('        let baseline = backend.shape_run("abc", &ShapingRequest::bootstrap("abc", run), face);', '        let baseline = backend\n            .shape_run("abc", &ShapingRequest::bootstrap("abc", run), face)\n            .unwrap();')
text = text.replace('        let shaped = backend.shape_run("abc", &configured, face);', '        let shaped = backend.shape_run("abc", &configured, face).unwrap();')
text = text.replace('        let shaped = FixedTextShaper::default().shape_run(\n            "ab",\n            &ShapingRequest::bootstrap("ab", run),\n            &face,\n        );', '        let shaped = FixedTextShaper::default()\n            .shape_run(\n                "ab",\n                &ShapingRequest::bootstrap("ab", run),\n                &face,\n            )\n            .unwrap();')
text = text.replace('        let shaped = FixedTextShaper::default().shape_run(\n            text,\n            &ShapingRequest::bootstrap(text, run),\n            fallback.face(run.face).unwrap(),\n        );', '        let shaped = FixedTextShaper::default()\n            .shape_run(\n                text,\n                &ShapingRequest::bootstrap(text, run),\n                fallback.face(run.face).unwrap(),\n            )\n            .unwrap();')
# Multi-run iterator test.
text = text.replace('                    .map(|face| backend.shape_run(&self.text, &request, face))', '                    .map(|face| backend.shape_run(&self.text, &request, face))')
# Known test map invocation in module.
text = text.replace('                backend.shape_run(\n                    text,\n                    &ShapingRequest::bootstrap(text, *run),\n                    fallback.face(run.face).unwrap(),\n                )\n', '                backend\n                    .shape_run(\n                        text,\n                        &ShapingRequest::bootstrap(text, *run),\n                        fallback.face(run.face).unwrap(),\n                    )\n                    .unwrap()\n')

# Append focused production-backend tests inside the existing test module before its final closing brace.
insert = r'''

    #[test]
    fn opentype_font_data_rejects_invalid_inputs() {
        assert_eq!(
            OpenTypeFontData::try_new(Arc::<[u8]>::from(&b"not-a-font"[..]), 0, 16.0),
            Err(ShapingError::InvalidOpenTypeData(FontFaceId::new(0)))
        );
        assert_eq!(
            OpenTypeFontData::try_new(
                Arc::<[u8]>::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING),
                0,
                0.0,
            ),
            Err(ShapingError::InvalidPixelsPerEm)
        );
    }

    fn production_face() -> FontFace {
        FontFace {
            id: FontFaceId::new(42),
            family: FontFamily("Noto Serif test subset".into()),
            coverage: FontCoverage::LastResort,
            metrics: FontMetrics {
                ascent: 14.0,
                descent: 4.0,
                line_gap: 0.0,
            },
            advance: 8.0,
        }
    }

    fn production_shaper(face: &FontFace) -> OpenTypeShaper {
        let data = OpenTypeFontData::try_new(
            Arc::<[u8]>::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING),
            0,
            16.0,
        )
        .unwrap();
        let mut backend = OpenTypeShaper::new();
        backend.insert_font(face.id, data);
        backend
    }

    #[test]
    fn production_opentype_shaper_forms_ligatures_with_source_ranges() {
        let face = production_face();
        let backend = production_shaper(&face);
        let run = ShapingRun {
            range: TextRange::new(0, 2),
            face: face.id,
            level: BidiLevel::new(0),
        };
        let shaped = backend
            .shape_run("fi", &ShapingRequest::bootstrap("fi", run), &face)
            .unwrap();

        assert_eq!(shaped.glyphs.len(), 1);
        assert_eq!(shaped.glyphs[0].source, TextRange::new(0, 2));
        assert!(shaped.glyphs[0].id.value() > 0);
        assert!(shaped.advance > 0.0);
    }

    #[test]
    fn production_opentype_shaper_applies_feature_metadata() {
        let face = production_face();
        let backend = production_shaper(&face);
        let run = ShapingRun {
            range: TextRange::new(0, 2),
            face: face.id,
            level: BidiLevel::new(0),
        };
        let mut request = ShapingRequest::bootstrap("fi", run);
        request.features.push(OpenTypeFeature {
            tag: OpenTypeTag::from_bytes(*b"liga"),
            value: 0,
        });
        let shaped = backend.shape_run("fi", &request, &face).unwrap();

        assert_eq!(shaped.glyphs.len(), 2);
        assert_eq!(shaped.glyphs[0].source, TextRange::new(0, 1));
        assert_eq!(shaped.glyphs[1].source, TextRange::new(1, 2));
    }

    #[test]
    fn production_opentype_shaper_fails_explicitly_without_registered_data() {
        let face = production_face();
        let backend = OpenTypeShaper::new();
        let run = ShapingRun {
            range: TextRange::new(0, 1),
            face: face.id,
            level: BidiLevel::new(0),
        };
        assert_eq!(
            backend.shape_run("f", &ShapingRequest::bootstrap("f", run), &face),
            Err(ShapingError::MissingOpenTypeData(face.id))
        );
    }
'''
last = text.rfind('\n}')
if last == -1:
    raise SystemExit("missing layout final brace")
text = text[:last] + insert + text[last:]
layout.write_text(text)

backlog = Path("docs/R1-BACKLOG.md")
text = backlog.read_text().replace(
    '- [ ] Connect a production OpenType shaping backend behind the existing shaping request boundary.\n',
    '- [x] Connect a production OpenType shaping backend behind the existing shaping request boundary.\n',
    1,
)
backlog.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
text = architecture.read_text()
marker = '## Image resource boundary\n'
section = r'''## Production OpenType shaping boundary

R1 keeps text segmentation, fallback selection and shaping metadata in Rarog-owned types while adding a real OpenType shaping implementation behind `ShapingBackend`. `OpenTypeShaper` owns validated immutable font bytes keyed by `FontFaceId`; HarfRust types do not cross the public shaping boundary. A production shaping call is explicitly fallible, because missing font data, invalid font bytes, invalid language metadata or invalid returned cluster mapping must not silently become synthetic glyphs.

The adapter submits each existing bidi×font×script shaping request with explicit direction, script, language, OpenType feature settings and variable-font coordinates. HarfRust cluster values are Rarog character indices, not UTF-8 byte offsets. The adapter reconstructs logical `TextRange` ownership from the sorted cluster starts so ligatures and RTL output retain deterministic source mapping even when glyph order is visual.

The current default `TextRun` geometry remains the deterministic fixed bootstrap shaper. R1 does not switch Web layout to production font metrics until the platform font-discovery layer can provide resolved real font faces. This keeps backend integration measurable without inventing a cross-platform default font policy. The first Windows font discovery/text adapter is the next production-text slice.

'''
if marker not in text:
    raise SystemExit("missing architecture image marker")
architecture.write_text(text.replace(marker, section + marker, 1))

Path("docs/adr/0033-production-opentype-shaper.md").write_text(r'''# ADR-0033: Production OpenType shaper behind the Rarog boundary

**Status:** Accepted

## Context

Rarog already splits text into grapheme-safe bidi, font and script shaping requests and exposes backend-neutral glyph IDs, advances, offsets and source ranges. The R0 `FixedTextShaper` deliberately ignores OpenType semantics. R1 needs a real production shaping implementation without leaking a third-party API into layout callers or coupling shaping to Windows font discovery.

## Decision

Use HarfRust behind the existing Rarog-owned `ShapingBackend` contract. Rarog owns immutable `OpenTypeFontData` containing font bytes, collection face index and pixels-per-em, while `OpenTypeShaper` maps those records to existing `FontFaceId` values. HarfRust types remain implementation details.

Make the shaping backend contract explicitly fallible. Missing faces/font data, invalid font data, invalid language metadata and invalid source clusters are errors rather than synthetic fallback output or panics. The fixed bootstrap backend returns successful deterministic output through the same result boundary.

The adapter forwards direction, script, language, whole-run OpenType feature settings and variation coordinates from `ShapingRequest`. Input characters are added with global Rarog character-index cluster values. Output cluster starts are sorted independently of visual glyph order and converted back into logical `TextRange` values, preserving source ownership for ligatures and RTL shaping.

Use a fixed 1/64-pixel position scale for the HarfRust adapter. Font metrics remain owned by the resolved Rarog `FontFace`; the upcoming platform font adapter is responsible for supplying production metrics and font data consistently.

## Consequences

- Complex OpenType substitution and positioning can be exercised now without changing segmentation or fragment identity.
- Feature and variation metadata cross a measured production backend instead of being ignored by every implementation.
- The backend can fail explicitly on malformed or absent font data.
- No system-font lookup, Windows API dependency or default-font policy enters the portable layout crate.
- Default Web layout remains on the fixed bootstrap geometry until platform font discovery is connected.
''')
