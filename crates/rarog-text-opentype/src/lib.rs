use harfrust::{
    BufferClusterLevel, Direction, Feature, FontRef, Language, ShapeOptions, ShaperData,
    ShaperInstance, Tag, UnicodeBuffer, Variation, script,
};
use rarog_layout::{
    FixedTextShaper, FontFace, FontFaceId, GlyphId, GlyphOffset, OpenTypeTag, PositionedGlyph,
    ShapedRun, ShapingBackend, ShapingRequest, ShapingScript, TextDirection, TextRange,
};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub const DEFAULT_MAX_FONT_FACES: usize = 256;
pub const DEFAULT_MAX_FONT_BYTES_PER_FACE: u64 = 64 * 1024 * 1024;
pub const DEFAULT_MAX_TOTAL_FONT_BYTES: u64 = 256 * 1024 * 1024;
const POSITION_FRACTIONAL_SCALE: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenTypeShapingLimits {
    pub max_faces: usize,
    pub max_bytes_per_face: u64,
    pub max_total_bytes: u64,
}

impl OpenTypeShapingLimits {
    pub const fn is_valid(self) -> bool {
        self.max_faces > 0
            && self.max_bytes_per_face > 0
            && self.max_total_bytes > 0
            && self.max_bytes_per_face <= self.max_total_bytes
    }
}

impl Default for OpenTypeShapingLimits {
    fn default() -> Self {
        Self {
            max_faces: DEFAULT_MAX_FONT_FACES,
            max_bytes_per_face: DEFAULT_MAX_FONT_BYTES_PER_FACE,
            max_total_bytes: DEFAULT_MAX_TOTAL_FONT_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpenTypeShapingError {
    InvalidLimits,
    InvalidFontSize(f32),
    EmptyFontData,
    FontByteCountOverflow,
    FaceLimitExceeded { faces: usize, limit: usize },
    FontByteLimitExceeded { bytes: u64, limit: u64 },
    TotalFontByteLimitExceeded { bytes: u64, limit: u64 },
    FaceAlreadyRegistered(FontFaceId),
    UnknownFace(FontFaceId),
    InvalidFont { face_index: u32 },
    InvalidTextRange(TextRange),
    InvalidVariationValue(OpenTypeTag),
    InvalidClusterBoundary(u32),
    InvalidScale(f32),
}

impl fmt::Display for OpenTypeShapingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("OpenType shaping limits are invalid"),
            Self::InvalidFontSize(size) => write!(formatter, "invalid font size {size}"),
            Self::EmptyFontData => formatter.write_str("font data must not be empty"),
            Self::FontByteCountOverflow => {
                formatter.write_str("font byte count does not fit in u64")
            }
            Self::FaceLimitExceeded { faces, limit } => write!(
                formatter,
                "OpenType shaping registry would contain {faces} faces; limit is {limit}"
            ),
            Self::FontByteLimitExceeded { bytes, limit } => write!(
                formatter,
                "OpenType font contains {bytes} bytes; per-face limit is {limit}"
            ),
            Self::TotalFontByteLimitExceeded { bytes, limit } => write!(
                formatter,
                "OpenType shaping registry would retain {bytes} bytes; limit is {limit}"
            ),
            Self::FaceAlreadyRegistered(id) => {
                write!(formatter, "font face {} is already registered", id.value())
            }
            Self::UnknownFace(id) => write!(formatter, "unknown font face {}", id.value()),
            Self::InvalidFont { face_index } => {
                write!(formatter, "invalid OpenType font face index {face_index}")
            }
            Self::InvalidTextRange(range) => write!(
                formatter,
                "invalid shaping text range {}..{}",
                range.start, range.end
            ),
            Self::InvalidVariationValue(axis) => write!(
                formatter,
                "variation axis 0x{:08x} has a non-finite value",
                axis.value()
            ),
            Self::InvalidClusterBoundary(cluster) => write!(
                formatter,
                "shaper returned non-character cluster boundary {cluster}"
            ),
            Self::InvalidScale(size) => {
                write!(
                    formatter,
                    "font size {size} cannot be represented at shaping precision"
                )
            }
        }
    }
}

impl std::error::Error for OpenTypeShapingError {}

struct RegisteredFont {
    data: Arc<[u8]>,
    face_index: u32,
    size_px: f32,
    shaper_data: ShaperData,
}

pub struct OpenTypeShapingBackend {
    limits: OpenTypeShapingLimits,
    total_bytes: u64,
    faces: BTreeMap<FontFaceId, RegisteredFont>,
}

impl Default for OpenTypeShapingBackend {
    fn default() -> Self {
        Self {
            limits: OpenTypeShapingLimits::default(),
            total_bytes: 0,
            faces: BTreeMap::new(),
        }
    }
}

impl OpenTypeShapingBackend {
    pub fn try_with_limits(limits: OpenTypeShapingLimits) -> Result<Self, OpenTypeShapingError> {
        if !limits.is_valid() {
            return Err(OpenTypeShapingError::InvalidLimits);
        }
        Ok(Self {
            limits,
            total_bytes: 0,
            faces: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> OpenTypeShapingLimits {
        self.limits
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    pub const fn total_font_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn contains_face(&self, id: FontFaceId) -> bool {
        self.faces.contains_key(&id)
    }

    pub fn register_face(
        &mut self,
        id: FontFaceId,
        data: Arc<[u8]>,
        face_index: u32,
        size_px: f32,
    ) -> Result<(), OpenTypeShapingError> {
        if !size_px.is_finite() || size_px <= 0.0 {
            return Err(OpenTypeShapingError::InvalidFontSize(size_px));
        }
        scale_for_size(size_px)?;
        if data.is_empty() {
            return Err(OpenTypeShapingError::EmptyFontData);
        }
        if self.faces.contains_key(&id) {
            return Err(OpenTypeShapingError::FaceAlreadyRegistered(id));
        }

        let new_face_count = self.faces.len().saturating_add(1);
        if new_face_count > self.limits.max_faces {
            return Err(OpenTypeShapingError::FaceLimitExceeded {
                faces: new_face_count,
                limit: self.limits.max_faces,
            });
        }

        let bytes =
            u64::try_from(data.len()).map_err(|_| OpenTypeShapingError::FontByteCountOverflow)?;
        if bytes > self.limits.max_bytes_per_face {
            return Err(OpenTypeShapingError::FontByteLimitExceeded {
                bytes,
                limit: self.limits.max_bytes_per_face,
            });
        }
        let total_bytes = self.total_bytes.checked_add(bytes).ok_or(
            OpenTypeShapingError::TotalFontByteLimitExceeded {
                bytes: u64::MAX,
                limit: self.limits.max_total_bytes,
            },
        )?;
        if total_bytes > self.limits.max_total_bytes {
            return Err(OpenTypeShapingError::TotalFontByteLimitExceeded {
                bytes: total_bytes,
                limit: self.limits.max_total_bytes,
            });
        }

        let font = FontRef::from_index(data.as_ref(), face_index)
            .map_err(|_| OpenTypeShapingError::InvalidFont { face_index })?;
        let shaper_data = ShaperData::new(&font);
        self.faces.insert(
            id,
            RegisteredFont {
                data,
                face_index,
                size_px,
                shaper_data,
            },
        );
        self.total_bytes = total_bytes;
        Ok(())
    }

    pub fn unregister_face(&mut self, id: FontFaceId) -> bool {
        let Some(removed) = self.faces.remove(&id) else {
            return false;
        };
        let bytes = u64::try_from(removed.data.len()).unwrap_or(self.total_bytes);
        self.total_bytes = self.total_bytes.saturating_sub(bytes);
        true
    }

    pub fn try_shape_run(
        &self,
        text: &str,
        request: &ShapingRequest,
        face: &FontFace,
    ) -> Result<ShapedRun, OpenTypeShapingError> {
        let registered = self
            .faces
            .get(&request.run.face)
            .ok_or(OpenTypeShapingError::UnknownFace(request.run.face))?;
        if face.id != request.run.face {
            return Err(OpenTypeShapingError::UnknownFace(request.run.face));
        }

        let scalar_boundaries = scalar_byte_boundaries(text);
        let start_byte = *scalar_boundaries
            .get(request.run.range.start)
            .ok_or(OpenTypeShapingError::InvalidTextRange(request.run.range))?;
        let end_byte = *scalar_boundaries
            .get(request.run.range.end)
            .ok_or(OpenTypeShapingError::InvalidTextRange(request.run.range))?;
        if request.run.range.start > request.run.range.end || start_byte > end_byte {
            return Err(OpenTypeShapingError::InvalidTextRange(request.run.range));
        }
        if start_byte == end_byte {
            return Ok(ShapedRun {
                run: request.run,
                glyphs: Vec::new(),
                advance: 0.0,
                metrics: face.metrics,
            });
        }

        let run_text = &text[start_byte..end_byte];
        let font =
            FontRef::from_index(registered.data.as_ref(), registered.face_index).map_err(|_| {
                OpenTypeShapingError::InvalidFont {
                    face_index: registered.face_index,
                }
            })?;

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(run_text);
        buffer.set_cluster_level(BufferClusterLevel::MonotoneGraphemes);
        buffer.set_direction(match request.run.direction() {
            TextDirection::Ltr => Direction::LeftToRight,
            TextDirection::Rtl => Direction::RightToLeft,
        });
        buffer.set_script(script_for_request(request.script));
        if let Some(language) = Language::new(request.language.as_str()) {
            buffer.set_language(language);
        }

        let features = request
            .features
            .iter()
            .map(|feature| Feature::new(tag(feature.tag), feature.value, ..))
            .collect::<Vec<_>>();
        let mut variations = Vec::with_capacity(request.variations.len());
        for variation in &request.variations {
            if !variation.value.is_finite() {
                return Err(OpenTypeShapingError::InvalidVariationValue(variation.axis));
            }
            variations.push(Variation {
                tag: tag(variation.axis),
                value: variation.value,
            });
        }
        let instance = ShaperInstance::from_variations(&font, variations);
        let shaper = registered
            .shaper_data
            .shaper(&font)
            .instance(Some(&instance))
            .build();
        let scale = scale_for_size(registered.size_px)?;
        let glyph_buffer = shaper.shape(
            buffer,
            ShapeOptions::new().scale(Some(scale)).features(&features),
        );

        let infos = glyph_buffer.glyph_infos();
        let positions = glyph_buffer.glyph_positions();
        let cluster_starts = cluster_starts(infos, run_text.len())?;
        let local_scalar_boundaries = scalar_byte_boundaries(run_text);
        let mut glyphs = Vec::with_capacity(infos.len());

        for (info, position) in infos.iter().zip(positions) {
            let source = cluster_source_range(
                info.cluster,
                &cluster_starts,
                &local_scalar_boundaries,
                request.run.range.start,
            )?;
            glyphs.push(PositionedGlyph {
                id: GlyphId::new(info.glyph_id),
                source,
                advance: position.x_advance as f32 / POSITION_FRACTIONAL_SCALE,
                offset: GlyphOffset {
                    x: position.x_offset as f32 / POSITION_FRACTIONAL_SCALE,
                    y: position.y_offset as f32 / POSITION_FRACTIONAL_SCALE,
                },
            });
        }

        let advance = glyphs.iter().map(|glyph| glyph.advance).sum();
        Ok(ShapedRun {
            run: request.run,
            glyphs,
            advance,
            metrics: face.metrics,
        })
    }
}

impl ShapingBackend for OpenTypeShapingBackend {
    fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun {
        self.try_shape_run(text, request, face)
            .unwrap_or_else(|_| FixedTextShaper::default().shape_run(text, request, face))
    }
}

fn scale_for_size(size_px: f32) -> Result<i32, OpenTypeShapingError> {
    let scaled = size_px * POSITION_FRACTIONAL_SCALE;
    if !scaled.is_finite() || scaled < 1.0 || scaled > i32::MAX as f32 {
        return Err(OpenTypeShapingError::InvalidScale(size_px));
    }
    Ok(scaled.round() as i32)
}

fn tag(value: OpenTypeTag) -> Tag {
    Tag::from_be_bytes(value.value().to_be_bytes())
}

fn script_for_request(value: ShapingScript) -> harfrust::Script {
    match value {
        ShapingScript::Common => script::COMMON,
        ShapingScript::Latin => script::LATIN,
        ShapingScript::Cyrillic => script::CYRILLIC,
        ShapingScript::Hebrew => script::HEBREW,
        ShapingScript::Arabic => script::ARABIC,
        ShapingScript::Han => script::HAN,
        ShapingScript::Emoji => harfrust::Script::from_iso15924_tag(Tag::from_be_bytes(*b"Zsye"))
            .unwrap_or(script::COMMON),
        ShapingScript::Unknown => script::UNKNOWN,
    }
}

fn scalar_byte_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = text
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(text.len());
    boundaries
}

fn cluster_starts(
    infos: &[harfrust::GlyphInfo],
    text_len: usize,
) -> Result<Vec<u32>, OpenTypeShapingError> {
    let text_len = u32::try_from(text_len)
        .map_err(|_| OpenTypeShapingError::InvalidClusterBoundary(u32::MAX))?;
    let mut starts = infos.iter().map(|info| info.cluster).collect::<Vec<_>>();
    if starts.iter().any(|cluster| *cluster >= text_len) && text_len != 0 {
        return Err(OpenTypeShapingError::InvalidClusterBoundary(
            starts
                .into_iter()
                .find(|cluster| *cluster >= text_len)
                .unwrap_or(text_len),
        ));
    }
    starts.sort_unstable();
    starts.dedup();
    starts.push(text_len);
    Ok(starts)
}

fn cluster_source_range(
    cluster: u32,
    starts: &[u32],
    scalar_boundaries: &[usize],
    global_scalar_start: usize,
) -> Result<TextRange, OpenTypeShapingError> {
    let cluster_index = starts
        .binary_search(&cluster)
        .map_err(|_| OpenTypeShapingError::InvalidClusterBoundary(cluster))?;
    let next = *starts
        .get(cluster_index.saturating_add(1))
        .ok_or(OpenTypeShapingError::InvalidClusterBoundary(cluster))?;
    let cluster_byte = usize::try_from(cluster)
        .map_err(|_| OpenTypeShapingError::InvalidClusterBoundary(cluster))?;
    let next_byte =
        usize::try_from(next).map_err(|_| OpenTypeShapingError::InvalidClusterBoundary(next))?;
    let local_start = scalar_boundaries
        .binary_search(&cluster_byte)
        .map_err(|_| OpenTypeShapingError::InvalidClusterBoundary(cluster))?;
    let local_end = scalar_boundaries
        .binary_search(&next_byte)
        .map_err(|_| OpenTypeShapingError::InvalidClusterBoundary(next))?;
    Ok(TextRange::new(
        global_scalar_start.saturating_add(local_start),
        global_scalar_start.saturating_add(local_end),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_layout::{BidiLevel, FontCoverage, FontFamily, FontMetrics, ShapingRun};

    fn fixture_face(id: FontFaceId) -> FontFace {
        FontFace {
            id,
            family: FontFamily("Noto Serif fixture".into()),
            coverage: FontCoverage::LatinCyrillic,
            metrics: FontMetrics {
                ascent: 12.0,
                descent: 4.0,
                line_gap: 0.0,
            },
            advance: 8.0,
        }
    }

    fn request(text: &str, id: FontFaceId) -> ShapingRequest {
        ShapingRequest::bootstrap(
            text,
            ShapingRun {
                range: TextRange::new(0, text.chars().count()),
                face: id,
                level: BidiLevel::new(0),
            },
        )
    }

    #[test]
    fn invalid_limits_are_rejected() {
        assert!(matches!(
            OpenTypeShapingBackend::try_with_limits(OpenTypeShapingLimits {
                max_faces: 0,
                max_bytes_per_face: 1,
                max_total_bytes: 1,
            }),
            Err(OpenTypeShapingError::InvalidLimits)
        ));
    }

    #[test]
    fn malformed_fonts_are_rejected_without_registration() {
        let mut backend = OpenTypeShapingBackend::default();
        let id = FontFaceId::new(7);
        assert!(matches!(
            backend.register_face(id, Arc::from(&b"not a font"[..]), 0, 16.0),
            Err(OpenTypeShapingError::InvalidFont { face_index: 0 })
        ));
        assert!(!backend.contains_face(id));
        assert_eq!(backend.total_font_bytes(), 0);
    }

    #[test]
    fn registration_limits_are_enforced_before_retaining_font_data() {
        let data: Arc<[u8]> = Arc::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING);
        let mut backend = OpenTypeShapingBackend::try_with_limits(OpenTypeShapingLimits {
            max_faces: 1,
            max_bytes_per_face: u64::try_from(data.len()).unwrap_or(u64::MAX),
            max_total_bytes: u64::try_from(data.len()).unwrap_or(u64::MAX),
        })
        .unwrap();
        backend
            .register_face(FontFaceId::new(1), data.clone(), 0, 16.0)
            .unwrap();
        assert!(matches!(
            backend.register_face(FontFaceId::new(2), data, 0, 16.0),
            Err(OpenTypeShapingError::FaceLimitExceeded { .. })
        ));
    }

    #[test]
    fn production_backend_returns_real_scaled_glyph_geometry() {
        let data: Arc<[u8]> = Arc::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING);
        let text = "office";
        let small_id = FontFaceId::new(10);
        let large_id = FontFaceId::new(11);
        let mut backend = OpenTypeShapingBackend::default();
        backend
            .register_face(small_id, data.clone(), 0, 12.0)
            .unwrap();
        backend.register_face(large_id, data, 0, 24.0).unwrap();

        let small_face = fixture_face(small_id);
        let large_face = fixture_face(large_id);
        let small = backend
            .try_shape_run(text, &request(text, small_id), &small_face)
            .unwrap();
        let large = backend
            .try_shape_run(text, &request(text, large_id), &large_face)
            .unwrap();

        assert!(!small.glyphs.is_empty());
        assert!(small.advance > 0.0);
        assert!(small.glyphs.iter().all(|glyph| {
            glyph.source.start < glyph.source.end && glyph.source.end <= text.chars().count()
        }));
        let ratio = large.advance / small.advance;
        assert!((ratio - 2.0).abs() < 0.05, "unexpected scale ratio {ratio}");
    }

    #[test]
    fn feature_and_language_metadata_reach_the_real_shaper() {
        let id = FontFaceId::new(12);
        let mut backend = OpenTypeShapingBackend::default();
        backend
            .register_face(
                id,
                Arc::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING),
                0,
                16.0,
            )
            .unwrap();
        let face = fixture_face(id);
        let mut configured = request("office", id);
        configured.language = rarog_layout::LanguageTag::new("en");
        configured.features.push(rarog_layout::OpenTypeFeature {
            tag: OpenTypeTag::from_bytes(*b"liga"),
            value: 0,
        });
        configured
            .variations
            .push(rarog_layout::VariationCoordinate {
                axis: OpenTypeTag::from_bytes(*b"wght"),
                value: 650.0,
            });

        let shaped = backend.try_shape_run("office", &configured, &face).unwrap();
        assert!(!shaped.glyphs.is_empty());
        assert!(shaped.advance > 0.0);
    }

    #[test]
    fn scalar_ranges_round_trip_through_byte_based_clusters() {
        let id = FontFaceId::new(13);
        let mut backend = OpenTypeShapingBackend::default();
        backend
            .register_face(
                id,
                Arc::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING),
                0,
                16.0,
            )
            .unwrap();
        let face = fixture_face(id);
        let text = "éa";
        let shaped = backend
            .try_shape_run(text, &request(text, id), &face)
            .unwrap();

        assert!(shaped.glyphs.iter().all(|glyph| glyph.source.end <= 2));
        assert!(shaped.glyphs.iter().any(|glyph| glyph.source.start == 0));
        assert!(shaped.glyphs.iter().any(|glyph| glyph.source.end == 2));
    }

    #[test]
    fn infallible_trait_boundary_falls_back_for_unregistered_faces() {
        let backend = OpenTypeShapingBackend::default();
        let face = fixture_face(FontFaceId::new(99));
        let configured = request("abc", face.id);
        let shaped = backend.shape_run("abc", &configured, &face);

        assert_eq!(shaped.glyphs.len(), 3);
        assert_eq!(shaped.advance, 24.0);
        assert_eq!(shaped.glyphs[0].id, GlyphId::new('a' as u32));
    }

    #[test]
    fn unregister_releases_accounted_bytes() {
        let id = FontFaceId::new(14);
        let data: Arc<[u8]> = Arc::from(font_test_data::NOTOSERIF_AUTOHINT_SHAPING);
        let bytes = u64::try_from(data.len()).unwrap_or(u64::MAX);
        let mut backend = OpenTypeShapingBackend::default();
        backend.register_face(id, data, 0, 16.0).unwrap();
        assert_eq!(backend.total_font_bytes(), bytes);
        assert!(backend.unregister_face(id));
        assert_eq!(backend.total_font_bytes(), 0);
        assert!(!backend.unregister_face(id));
    }
}
