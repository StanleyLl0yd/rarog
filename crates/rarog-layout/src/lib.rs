mod flex;
mod grid;

pub use flex::{
    FlexContentAlignment, FlexCrossAlignment, FlexCrossSizeMetadata, FlexLayoutError,
    FlexMainAlignment, FlexMultiLineLayout, FlexRowItem, FlexRowLayout, FlexRowOptions,
    FlexRowPlacement, FlexibleFlexRowItem, layout_flexible_single_line_flex_row,
    layout_flexible_single_line_flex_row_with_alignment,
    layout_flexible_single_line_flex_row_with_item_alignments,
    layout_flexible_single_line_flex_row_with_options, layout_single_line_flex_row,
    layout_single_line_flex_row_with_alignment, layout_single_line_flex_row_with_item_alignments,
    layout_single_line_flex_row_with_options, layout_wrapped_flexible_rows_with_cross_metadata,
    layout_wrapped_flexible_rows_with_item_alignments,
};

pub use grid::{
    GridAxis, GridItem, GridLayout, GridLayoutError, GridPlacement, GridPlacementRequest, GridTrack,
    layout_fixed_grid, layout_fixed_grid_with_auto_placement,
};

use rarog_css::{
    AlignContent, AlignItems, AlignSelf, ComputedStyle, FlexDirection, FlexWrap, JustifyContent,
    StyleSet, VerticalAlign, computed_style_with_parent,
};
use rarog_dom::{Document, NodeId, NodeKind};
use rarog_types::{Point, Rect, Size};
use unicode_bidi::BidiInfo;
use unicode_linebreak::{BreakOpportunity as UnicodeBreakOpportunity, linebreaks};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutNodeId(usize);

impl LayoutNodeId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FragmentId(usize);

impl FragmentId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FragmentOrdinal(u32);

impl FragmentOrdinal {
    pub const fn index(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineBox {
    pub ordinal: u32,
    pub rect: Rect,
    pub text_range: TextRange,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlyphId(u32);

impl GlyphId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphOffset {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    pub id: GlyphId,
    pub source: TextRange,
    pub advance: f32,
    pub offset: GlyphOffset,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRun {
    pub run: ShapingRun,
    pub glyphs: Vec<PositionedGlyph>,
    pub advance: f32,
    pub metrics: FontMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpenTypeTag(u32);

impl OpenTypeTag {
    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(u32::from_be_bytes(bytes))
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapingScript {
    Common,
    Latin,
    Cyrillic,
    Hebrew,
    Arabic,
    Han,
    Emoji,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageTag(String);

impl LanguageTag {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self(if value.trim().is_empty() {
            "und".into()
        } else {
            value.to_ascii_lowercase()
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LanguageTag {
    fn default() -> Self {
        Self("und".into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenTypeFeature {
    pub tag: OpenTypeTag,
    pub value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariationCoordinate {
    pub axis: OpenTypeTag,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapingRequest {
    pub run: ShapingRun,
    pub script: ShapingScript,
    pub language: LanguageTag,
    pub features: Vec<OpenTypeFeature>,
    pub variations: Vec<VariationCoordinate>,
}

impl ShapingRequest {
    pub fn bootstrap(text: &str, run: ShapingRun) -> Self {
        Self {
            run,
            script: shaping_script_for_range(text, run.range),
            language: LanguageTag::default(),
            features: Vec::new(),
            variations: Vec::new(),
        }
    }
}

pub trait ShapingBackend {
    fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontFaceId(u16);

impl FontFaceId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFamily(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontCoverage {
    LatinCyrillic,
    HebrewArabic,
    Cjk,
    Emoji,
    LastResort,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontFace {
    pub id: FontFaceId,
    pub family: FontFamily,
    pub coverage: FontCoverage,
    pub metrics: FontMetrics,
    pub advance: f32,
}

impl FontFace {
    pub fn covers(&self, character: char) -> bool {
        let code = character as u32;
        if is_grapheme_extend(character)
            || character.is_whitespace()
            || character.is_ascii_punctuation()
            || matches!(code, 0x2000..=0x206f)
        {
            return true;
        }
        match self.coverage {
            FontCoverage::LatinCyrillic => {
                character.is_ascii_alphanumeric()
                    || matches!(code, 0x00a0..=0x024f | 0x0370..=0x052f)
            }
            FontCoverage::HebrewArabic => {
                matches!(code, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfefc)
            }
            FontCoverage::Cjk => {
                matches!(code, 0x2e80..=0x9fff | 0xac00..=0xd7af | 0xf900..=0xfaff)
            }
            FontCoverage::Emoji => {
                is_extended_pictographic(character) || is_regional_indicator(character)
            }
            FontCoverage::LastResort => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontFallbackChain {
    pub faces: Vec<FontFace>,
}

impl Default for FontFallbackChain {
    fn default() -> Self {
        let metrics = FontMetrics {
            ascent: 14.0,
            descent: 4.0,
            line_gap: 0.0,
        };
        Self {
            faces: vec![
                FontFace {
                    id: FontFaceId::new(0),
                    family: FontFamily("Rarog Sans".into()),
                    coverage: FontCoverage::LatinCyrillic,
                    metrics,
                    advance: 8.0,
                },
                FontFace {
                    id: FontFaceId::new(1),
                    family: FontFamily("Rarog RTL".into()),
                    coverage: FontCoverage::HebrewArabic,
                    metrics,
                    advance: 8.0,
                },
                FontFace {
                    id: FontFaceId::new(2),
                    family: FontFamily("Rarog CJK".into()),
                    coverage: FontCoverage::Cjk,
                    metrics,
                    advance: 8.0,
                },
                FontFace {
                    id: FontFaceId::new(3),
                    family: FontFamily("Rarog Emoji".into()),
                    coverage: FontCoverage::Emoji,
                    metrics,
                    advance: 8.0,
                },
                FontFace {
                    id: FontFaceId::new(4),
                    family: FontFamily("Rarog LastResort".into()),
                    coverage: FontCoverage::LastResort,
                    metrics,
                    advance: 8.0,
                },
            ],
        }
    }
}

impl FontFallbackChain {
    pub fn face(&self, id: FontFaceId) -> Option<&FontFace> {
        self.faces.iter().find(|face| face.id == id)
    }

    pub fn select_face_for_range(&self, text: &str, range: TextRange) -> Option<FontFaceId> {
        let characters = text.chars().collect::<Vec<_>>();
        self.select_face_for_characters(&characters, range)
    }

    fn select_face_for_characters(
        &self,
        characters: &[char],
        range: TextRange,
    ) -> Option<FontFaceId> {
        let slice = characters.get(range.start..range.end)?;
        self.faces
            .iter()
            .find(|face| {
                slice
                    .iter()
                    .copied()
                    .all(|character| face.covers(character))
            })
            .map(|face| face.id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontRun {
    pub range: TextRange,
    pub face: FontFaceId,
}

fn is_common_font_character(character: char) -> bool {
    let code = character as u32;
    is_grapheme_extend(character)
        || character.is_whitespace()
        || character.is_ascii_punctuation()
        || matches!(code, 0x2000..=0x206f)
}

pub fn font_runs(text: &str, chain: &FontFallbackChain) -> Vec<FontRun> {
    let characters = text.chars().collect::<Vec<_>>();
    let boundaries = grapheme_boundaries(text);
    font_runs_for_segments(&characters, &boundaries, chain)
}

fn font_runs_for_segments(
    characters: &[char],
    boundaries: &[usize],
    chain: &FontFallbackChain,
) -> Vec<FontRun> {
    if boundaries.len() < 2 {
        return Vec::new();
    }

    let mut runs: Vec<FontRun> = Vec::new();
    for window in boundaries.windows(2) {
        let range = TextRange::new(window[0], window[1]);
        let common = characters[range.start..range.end]
            .iter()
            .copied()
            .all(is_common_font_character);
        let inherited = common && !runs.is_empty();
        let face = if inherited {
            runs.last().map(|run| run.face)
        } else {
            chain.select_face_for_characters(characters, range)
        }
        .or_else(|| chain.faces.last().map(|face| face.id));
        let Some(face) = face else {
            return Vec::new();
        };
        if let Some(previous) = runs.last_mut() {
            if previous.face == face && previous.range.end == range.start {
                previous.range.end = range.end;
                continue;
            }
        }
        runs.push(FontRun { range, face });
    }
    runs
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

impl ShapingBackend for FixedTextShaper {
    fn shape_run(&self, text: &str, request: &ShapingRequest, face: &FontFace) -> ShapedRun {
        let run = request.run;
        let characters = text.chars().collect::<Vec<_>>();
        let boundaries = grapheme_boundaries(text);
        let mut glyphs = boundaries
            .windows(2)
            .filter_map(|window| {
                let start = window[0];
                let end = window[1];
                if start < run.range.start || end > run.range.end {
                    return None;
                }
                let slice = &characters[start..end];
                let mandatory = slice.iter().copied().any(is_mandatory_break);
                let glyph_id = slice
                    .first()
                    .copied()
                    .map(|character| GlyphId::new(character as u32))
                    .unwrap_or_else(|| GlyphId::new(0));
                Some(PositionedGlyph {
                    id: glyph_id,
                    source: TextRange::new(start, end),
                    advance: if mandatory { 0.0 } else { face.advance },
                    offset: GlyphOffset::default(),
                })
            })
            .collect::<Vec<_>>();
        if run.direction() == TextDirection::Rtl {
            glyphs.reverse();
        }
        ShapedRun {
            advance: glyphs.iter().map(|glyph| glyph.advance).sum(),
            glyphs,
            run,
            metrics: face.metrics,
        }
    }
}

impl TextShaper for FixedTextShaper {
    fn shape(&self, text: &str) -> ShapedText {
        let characters = text.chars().collect::<Vec<_>>();
        let boundaries = grapheme_boundaries(text);
        shape_fixed_text(&characters, &boundaries, self.advance, self.metrics)
    }
}

fn shape_fixed_text(
    characters: &[char],
    boundaries: &[usize],
    advance: f32,
    metrics: FontMetrics,
) -> ShapedText {
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
                advance: if mandatory { 0.0 } else { advance },
            }
        })
        .collect::<Vec<_>>();
    ShapedText {
        advance: clusters.iter().map(|cluster| cluster.advance).sum(),
        clusters,
        metrics,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntrinsicSizes {
    pub min_content: f32,
    pub max_content: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub shaped: ShapedText,
    pub font_runs: Vec<FontRun>,
    pub advance: f32,
    pub line_height: f32,
}

impl TextRun {
    pub fn new(text: String) -> Self {
        Self::with_fallback(text, &FontFallbackChain::default())
    }

    pub fn with_fallback(text: String, fallback: &FontFallbackChain) -> Self {
        let shaper = FixedTextShaper::default();
        let characters = text.chars().collect::<Vec<_>>();
        let boundaries = grapheme_boundaries(&text);
        let shaped = shape_fixed_text(&characters, &boundaries, shaper.advance, shaper.metrics);
        let font_runs = font_runs_for_segments(&characters, &boundaries, fallback);
        Self {
            text,
            advance: shaped.advance,
            line_height: shaped.metrics.line_height(),
            shaped,
            font_runs,
        }
    }

    pub fn shaping_runs(&self) -> Vec<ShapingRun> {
        shaping_runs_for_font_runs(&self.text, &self.font_runs)
    }

    pub fn shaping_requests(&self) -> Vec<ShapingRequest> {
        shaping_requests_for_runs(&self.text, &self.shaping_runs())
    }

    pub fn shape_with_backend<B: ShapingBackend>(
        &self,
        fallback: &FontFallbackChain,
        backend: &B,
    ) -> Vec<ShapedRun> {
        self.shaping_requests()
            .into_iter()
            .filter_map(|request| {
                fallback
                    .face(request.run.face)
                    .map(|face| backend.shape_run(&self.text, &request, face))
            })
            .collect()
    }

    pub fn character_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn advance_for_range(&self, range: TextRange) -> f32 {
        self.shaped
            .clusters
            .iter()
            .filter(|cluster| {
                cluster.source.start >= range.start && cluster.source.end <= range.end
            })
            .map(|cluster| cluster.advance)
            .sum()
    }

    pub fn intrinsic_sizes(&self) -> IntrinsicSizes {
        let advance = FixedTextShaper::default().advance;
        let longest_word = self
            .text
            .split_whitespace()
            .map(|word| UnicodeSegmentation::graphemes(word, true).count() as f32 * advance)
            .fold(0.0, f32::max);
        IntrinsicSizes {
            min_content: longest_word,
            max_content: self.advance,
        }
    }
}

pub trait LineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BidiLevel(u8);

impl BidiLevel {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn direction(self) -> TextDirection {
        if self.0 % 2 == 0 {
            TextDirection::Ltr
        } else {
            TextDirection::Rtl
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BidiRun {
    pub range: TextRange,
    pub level: BidiLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapingRun {
    pub range: TextRange,
    pub face: FontFaceId,
    pub level: BidiLevel,
}

impl ShapingRun {
    pub const fn direction(self) -> TextDirection {
        self.level.direction()
    }
}

pub fn shaping_runs(text: &str, fallback: &FontFallbackChain) -> Vec<ShapingRun> {
    let fonts = font_runs(text, fallback);
    shaping_runs_for_font_runs(text, &fonts)
}

fn shaping_runs_for_font_runs(text: &str, fonts: &[FontRun]) -> Vec<ShapingRun> {
    let bidi = bidi_runs(text);
    let boundaries = grapheme_boundaries(text);
    let mut runs: Vec<ShapingRun> = Vec::new();
    let mut bidi_index = 0usize;
    let mut font_index = 0usize;

    while bidi_index < bidi.len() && font_index < fonts.len() {
        let bidi_run = bidi[bidi_index];
        let font_run = fonts[font_index];
        let start = bidi_run.range.start.max(font_run.range.start);
        let end = bidi_run.range.end.min(font_run.range.end);

        if start < end {
            debug_assert!(boundaries.binary_search(&start).is_ok());
            debug_assert!(boundaries.binary_search(&end).is_ok());
            let run = ShapingRun {
                range: TextRange::new(start, end),
                face: font_run.face,
                level: bidi_run.level,
            };
            if let Some(previous) = runs.last_mut() {
                if previous.face == run.face
                    && previous.level == run.level
                    && previous.range.end == run.range.start
                {
                    previous.range.end = run.range.end;
                } else {
                    runs.push(run);
                }
            } else {
                runs.push(run);
            }
        }

        if bidi_run.range.end <= font_run.range.end {
            bidi_index += 1;
        }
        if font_run.range.end <= bidi_run.range.end {
            font_index += 1;
        }
    }

    runs
}

fn shaping_requests_for_runs(text: &str, runs: &[ShapingRun]) -> Vec<ShapingRequest> {
    let characters = text.chars().collect::<Vec<_>>();
    let boundaries = grapheme_boundaries(text);
    let mut requests = Vec::new();

    for run in runs.iter().copied() {
        let mut request_start = run.range.start;
        let mut current_script = None;
        let first_boundary = boundaries.partition_point(|boundary| *boundary < run.range.start);
        let after_last_boundary = boundaries.partition_point(|boundary| *boundary <= run.range.end);

        for window in boundaries[first_boundary..after_last_boundary].windows(2) {
            let cluster_start = window[0];
            let cluster_end = window[1];
            let cluster_script = shaping_script_for_characters(
                &characters,
                TextRange::new(cluster_start, cluster_end),
            );
            if matches!(cluster_script, ShapingScript::Common) {
                continue;
            }

            match current_script {
                Some(script) if script != cluster_script => {
                    let request_run = ShapingRun {
                        range: TextRange::new(request_start, cluster_start),
                        face: run.face,
                        level: run.level,
                    };
                    requests.push(shaping_request(request_run, script));
                    request_start = cluster_start;
                    current_script = Some(cluster_script);
                }
                None => current_script = Some(cluster_script),
                Some(_) => {}
            }
        }

        if request_start < run.range.end {
            let request_run = ShapingRun {
                range: TextRange::new(request_start, run.range.end),
                face: run.face,
                level: run.level,
            };
            let script = current_script
                .unwrap_or_else(|| shaping_script_for_characters(&characters, request_run.range));
            requests.push(shaping_request(request_run, script));
        }
    }

    requests
}

fn shaping_request(run: ShapingRun, script: ShapingScript) -> ShapingRequest {
    ShapingRequest {
        run,
        script,
        language: LanguageTag::default(),
        features: Vec::new(),
        variations: Vec::new(),
    }
}

pub fn shaping_script_for_range(text: &str, range: TextRange) -> ShapingScript {
    let characters = text.chars().collect::<Vec<_>>();
    shaping_script_for_characters(&characters, range)
}

fn shaping_script_for_characters(characters: &[char], range: TextRange) -> ShapingScript {
    let Some(slice) = characters.get(range.start..range.end) else {
        return ShapingScript::Unknown;
    };
    slice
        .iter()
        .copied()
        .find_map(shaping_script_for_character)
        .unwrap_or(ShapingScript::Common)
}

fn shaping_script_for_character(character: char) -> Option<ShapingScript> {
    if is_extended_pictographic(character) || is_regional_indicator(character) {
        return Some(ShapingScript::Emoji);
    }
    match character.script() {
        Script::Common | Script::Inherited => None,
        Script::Latin => Some(ShapingScript::Latin),
        Script::Cyrillic => Some(ShapingScript::Cyrillic),
        Script::Hebrew => Some(ShapingScript::Hebrew),
        Script::Arabic => Some(ShapingScript::Arabic),
        Script::Han => Some(ShapingScript::Han),
        Script::Unknown => Some(ShapingScript::Unknown),
        _ => Some(ShapingScript::Unknown),
    }
}

pub fn paragraph_direction(text: &str) -> TextDirection {
    BidiInfo::new(text, None)
        .paragraphs
        .first()
        .map(|paragraph| {
            if paragraph.level.is_rtl() {
                TextDirection::Rtl
            } else {
                TextDirection::Ltr
            }
        })
        .unwrap_or(TextDirection::Ltr)
}

pub fn bidi_runs(text: &str) -> Vec<BidiRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let bidi = BidiInfo::new(text, None);
    let mut runs = Vec::new();
    let mut current_level = None;
    let mut run_start = 0usize;
    let mut character_index = 0usize;

    for (byte_index, _) in text.char_indices() {
        let level = BidiLevel::new(bidi.levels[byte_index].number());
        match current_level {
            Some(current) if current != level => {
                runs.push(BidiRun {
                    range: TextRange::new(run_start, character_index),
                    level: current,
                });
                run_start = character_index;
                current_level = Some(level);
            }
            None => current_level = Some(level),
            Some(_) => {}
        }
        character_index += 1;
    }

    if let Some(level) = current_level {
        runs.push(BidiRun {
            range: TextRange::new(run_start, character_index),
            level,
        });
    }
    runs
}

pub fn visual_bidi_runs(text: &str) -> Vec<BidiRun> {
    let mut runs = bidi_runs(text);
    if runs.is_empty() {
        return runs;
    }
    let max_level = runs.iter().map(|run| run.level.value()).max().unwrap_or(0);
    let min_odd = runs
        .iter()
        .map(|run| run.level.value())
        .filter(|level| level % 2 == 1)
        .min();
    if let Some(min_odd) = min_odd {
        for level in (min_odd..=max_level).rev() {
            let mut index = 0usize;
            while index < runs.len() {
                if runs[index].level.value() < level {
                    index += 1;
                    continue;
                }
                let start = index;
                while index < runs.len() && runs[index].level.value() >= level {
                    index += 1;
                }
                runs[start..index].reverse();
            }
        }
    }
    runs
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakKind {
    Soft,
    Mandatory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakOpportunity {
    pub index: usize,
    pub kind: BreakKind,
}

pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::new();
    let mut character_index = 0usize;
    boundaries.push(0);
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        character_index = character_index.saturating_add(grapheme.chars().count());
        boundaries.push(character_index);
    }
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

pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    let mut opportunities = Vec::new();
    let mut previous_byte = 0usize;
    let mut character_index = 0usize;
    let terminal_is_explicit_break = text.chars().last().is_some_and(is_mandatory_break);

    for (byte_index, opportunity) in linebreaks(text) {
        character_index =
            character_index.saturating_add(text[previous_byte..byte_index].chars().count());
        previous_byte = byte_index;
        if byte_index == text.len() && !terminal_is_explicit_break {
            continue;
        }
        opportunities.push(BreakOpportunity {
            index: character_index,
            kind: match opportunity {
                UnicodeBreakOpportunity::Allowed => BreakKind::Soft,
                UnicodeBreakOpportunity::Mandatory => BreakKind::Mandatory,
            },
        });
    }
    opportunities
}

fn is_mandatory_break(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn is_non_breaking_boundary(characters: &[char], index: usize) -> bool {
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index);
    previous
        .into_iter()
        .chain(next)
        .any(|character| matches!(character, '\u{00a0}' | '\u{202f}'))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnicodeLineBreaker;

impl UnicodeLineBreaker {
    fn break_text_with_widths(
        &self,
        run: &TextRun,
        first_line_width: f32,
        following_line_width: f32,
    ) -> Vec<TextRange> {
        self.break_text_with_widths_and_terminal_reserve(
            run,
            first_line_width,
            following_line_width,
            0.0,
        )
    }

    fn break_text_with_widths_and_terminal_reserve(
        &self,
        run: &TextRun,
        first_line_width: f32,
        following_line_width: f32,
        terminal_reserve: f32,
    ) -> Vec<TextRange> {
        if run.shaped.clusters.is_empty() {
            return vec![TextRange::new(0, 0)];
        }

        let characters = run.text.chars().collect::<Vec<_>>();
        let character_count = characters.len();
        let boundaries = grapheme_boundaries(&run.text);
        let opportunities = unicode_break_opportunities(&run.text);
        let mut prefix_advance = vec![0.0f32; characters.len().saturating_add(1)];
        for cluster in &run.shaped.clusters {
            prefix_advance[cluster.source.end] =
                prefix_advance[cluster.source.start] + cluster.advance;
        }
        let mut ranges = Vec::new();
        let mut line_start = 0;
        let mut last_soft = None;
        let mut width = 0.0;
        let mut line_limit = first_line_width;

        for cluster in &run.shaped.clusters {
            width += cluster.advance;
            let boundary = cluster.source.end;
            let opportunity = opportunities
                .binary_search_by_key(&boundary, |opportunity| opportunity.index)
                .ok()
                .map(|index| opportunities[index]);

            if matches!(
                opportunity.map(|value| value.kind),
                Some(BreakKind::Mandatory)
            ) {
                ranges.push(TextRange::new(line_start, boundary));
                line_start = boundary;
                last_soft = None;
                width = 0.0;
                line_limit = following_line_width;
                continue;
            }

            let terminal = if boundary == character_count {
                terminal_reserve
            } else {
                0.0
            };
            if line_limit.is_finite() && line_limit >= 0.0 && width + terminal > line_limit {
                let emergency = cluster.source.start;
                let break_at = last_soft.filter(|value| *value > line_start).or_else(|| {
                    (emergency > line_start
                        && boundaries.binary_search(&emergency).is_ok()
                        && !is_non_breaking_boundary(&characters, emergency))
                    .then_some(emergency)
                });
                if let Some(break_at) = break_at {
                    ranges.push(TextRange::new(line_start, break_at));
                    line_start = break_at;
                    width = prefix_advance[boundary] - prefix_advance[line_start];
                    line_limit = following_line_width;
                    let end = opportunities.partition_point(|value| value.index < boundary);
                    last_soft = opportunities[..end]
                        .iter()
                        .rev()
                        .find(|value| value.kind == BreakKind::Soft && value.index > line_start)
                        .map(|value| value.index);
                }
            }

            if matches!(opportunity.map(|value| value.kind), Some(BreakKind::Soft))
                && boundary > line_start
            {
                last_soft = Some(boundary);
            }
        }

        if line_start < character_count {
            ranges.push(TextRange::new(line_start, character_count));
        }
        if ranges.is_empty() {
            ranges.push(TextRange::new(0, 0));
        }
        ranges
    }
}

impl LineBreaker for UnicodeLineBreaker {
    fn break_text(&self, run: &TextRun, available_width: f32) -> Vec<TextRange> {
        self.break_text_with_widths(run, available_width, available_width)
    }
}

pub type FixedAdvanceLineBreaker = UnicodeLineBreaker;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainingBlock {
    pub origin: Point,
    pub available: Size,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutNodeKind {
    Root,
    Box,
    Text(TextRun),
}

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub id: LayoutNodeId,
    pub dom_node: Option<NodeId>,
    pub kind: LayoutNodeKind,
    pub style: ComputedStyle,
    pub intrinsic: IntrinsicSizes,
    pub children: Vec<LayoutNode>,
    margin_collapse_boundary: bool,
}

#[derive(Clone, Debug)]
pub struct LayoutTree {
    pub root: LayoutNode,
}

impl LayoutTree {
    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        snapshot_layout_node(&self.root, 0, &mut output);
        output
    }

    pub fn style_snapshot(&self) -> String {
        let mut output = String::new();
        snapshot_style_node(&self.root, &mut output);
        output
    }

    pub fn node_count(&self) -> usize {
        count_layout_nodes(&self.root)
    }
}

fn count_layout_nodes(node: &LayoutNode) -> usize {
    1 + node.children.iter().map(count_layout_nodes).sum::<usize>()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxModel {
    pub margin_box: Rect,
    pub border_box: Rect,
    pub padding_box: Rect,
    pub content_box: Rect,
}

impl BoxModel {
    pub const fn single(rect: Rect) -> Self {
        Self {
            margin_box: rect,
            border_box: rect,
            padding_box: rect,
            content_box: rect,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentKind {
    Root,
    Box,
    Text,
}

#[derive(Clone, Debug)]
pub struct Fragment {
    pub id: FragmentId,
    pub ordinal: FragmentOrdinal,
    pub layout_node: LayoutNodeId,
    pub dom_node: Option<NodeId>,
    pub kind: FragmentKind,
    pub boxes: BoxModel,
    pub style: ComputedStyle,
    pub text_range: Option<TextRange>,
    pub line_box: Option<LineBox>,
    pub children: Vec<Fragment>,
}

#[derive(Clone, Debug)]
pub struct FragmentTree {
    pub root: Fragment,
}

impl FragmentTree {
    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        snapshot_fragment(&self.root, 0, &mut output);
        output
    }

    pub fn fragment_count(&self) -> usize {
        count_fragments(&self.root)
    }
}

fn count_fragments(fragment: &Fragment) -> usize {
    1 + fragment.children.iter().map(count_fragments).sum::<usize>()
}

#[derive(Clone, Debug)]
pub struct LayoutOutput {
    pub tree: LayoutTree,
    pub fragments: FragmentTree,
}

pub fn layout_document(doc: &Document, viewport: Size) -> LayoutOutput {
    let styles = StyleSet::for_document(doc);
    layout_document_with_styles(doc, &styles, viewport)
}

pub fn build_layout_tree(doc: &Document, styles: &StyleSet) -> LayoutTree {
    let mut tree_builder = LayoutTreeBuilder::new(styles);
    let root = tree_builder
        .build_node(doc, doc.root(), None)
        .expect("document root always creates a layout root");
    LayoutTree { root }
}

pub fn layout_document_with_styles(
    doc: &Document,
    styles: &StyleSet,
    viewport: Size,
) -> LayoutOutput {
    let tree = build_layout_tree(doc, styles);
    let fragments = relayout_tree(&tree, viewport);
    LayoutOutput { tree, fragments }
}

pub fn relayout_tree(tree: &LayoutTree, viewport: Size) -> FragmentTree {
    let mut fragment_builder = FragmentBuilder::default();
    fragment_builder.build(tree, viewport)
}

pub fn refresh_text_node(tree: &mut LayoutTree, document: &Document, dom_node: NodeId) -> bool {
    let Some(NodeKind::Text(text)) = document.node(dom_node).map(|node| &node.kind) else {
        return false;
    };
    refresh_text_node_recursive(&mut tree.root, dom_node, text)
}

fn refresh_text_node_recursive(node: &mut LayoutNode, dom_node: NodeId, text: &str) -> bool {
    if node.dom_node == Some(dom_node) {
        if !matches!(node.kind, LayoutNodeKind::Text(_)) {
            return false;
        }
        node.kind = LayoutNodeKind::Text(TextRun::new(text.to_owned()));
        node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
        return true;
    }

    let changed = node
        .children
        .iter_mut()
        .any(|child| refresh_text_node_recursive(child, dom_node, text));
    if changed {
        node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
    }
    changed
}

pub fn refresh_layout_subtree(
    tree: &mut LayoutTree,
    document: &Document,
    styles: &StyleSet,
    dom_node: NodeId,
) -> bool {
    refresh_layout_subtrees(tree, document, styles, &[dom_node])
}

pub fn refresh_layout_subtrees(
    tree: &mut LayoutTree,
    document: &Document,
    styles: &StyleSet,
    dom_nodes: &[NodeId],
) -> bool {
    if dom_nodes.is_empty() {
        return false;
    }

    let mut retained_ids = std::collections::BTreeMap::new();
    collect_layout_node_ids(&tree.root, &mut retained_ids);
    let mut next_id = max_layout_node_id(&tree.root).saturating_add(1);
    for dom_node in dom_nodes {
        if !refresh_layout_subtree_recursive(
            &mut tree.root,
            document,
            styles,
            *dom_node,
            None,
            &retained_ids,
            &mut next_id,
        ) {
            return false;
        }
    }
    true
}

fn refresh_layout_subtree_recursive(
    node: &mut LayoutNode,
    document: &Document,
    styles: &StyleSet,
    dom_node: NodeId,
    parent_style: Option<ComputedStyle>,
    retained_ids: &std::collections::BTreeMap<NodeId, LayoutNodeId>,
    next_id: &mut usize,
) -> bool {
    if node.dom_node == Some(dom_node) {
        let mut builder = LayoutTreeBuilder {
            next_id: *next_id,
            styles,
        };
        let Some(mut replacement) = builder.build_node(document, dom_node, parent_style) else {
            return false;
        };
        *next_id = builder.next_id;
        reuse_layout_node_ids(&mut replacement, retained_ids);
        *node = replacement;
        return true;
    }

    let style = node.style;
    for child in &mut node.children {
        if refresh_layout_subtree_recursive(
            child,
            document,
            styles,
            dom_node,
            Some(style),
            retained_ids,
            next_id,
        ) {
            node.intrinsic = intrinsic_sizes_for_node(&node.kind, node.style, &node.children);
            return true;
        }
    }
    false
}

fn max_layout_node_id(node: &LayoutNode) -> usize {
    node.children
        .iter()
        .map(max_layout_node_id)
        .fold(node.id.index(), usize::max)
}

fn collect_layout_node_ids(
    node: &LayoutNode,
    ids: &mut std::collections::BTreeMap<NodeId, LayoutNodeId>,
) {
    if let Some(dom_node) = node.dom_node {
        ids.insert(dom_node, node.id);
    }
    for child in &node.children {
        collect_layout_node_ids(child, ids);
    }
}

fn reuse_layout_node_ids(
    node: &mut LayoutNode,
    ids: &std::collections::BTreeMap<NodeId, LayoutNodeId>,
) {
    if let Some(dom_node) = node.dom_node {
        if let Some(id) = ids.get(&dom_node) {
            node.id = *id;
        }
    }
    for child in &mut node.children {
        reuse_layout_node_ids(child, ids);
    }
}

pub fn fragment_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Option<&Fragment> {
    find_fragment(&tree.root, dom_node)
}

pub fn fragments_for_dom(tree: &FragmentTree, dom_node: NodeId) -> Vec<&Fragment> {
    let mut fragments = Vec::new();
    collect_fragments(&tree.root, dom_node, &mut fragments);
    fragments
}

pub fn relayout_fragment_subtree(
    tree: &LayoutTree,
    fragments: &mut FragmentTree,
    dom_node: NodeId,
) -> bool {
    let Some(layout_node) = find_layout_node(&tree.root, dom_node) else {
        return false;
    };
    let next_id = max_fragment_id(&fragments.root).saturating_add(1);
    let mut builder = FragmentBuilder {
        next_id,
        ..FragmentBuilder::default()
    };
    builder.prepare_margin_profiles(&tree.root);
    relayout_fragment_child(&mut fragments.root, layout_node, dom_node, &mut builder)
}

pub fn fragment_flow_start_index(
    tree: &LayoutTree,
    fragments: &FragmentTree,
    dirty_nodes: &[NodeId],
) -> Option<usize> {
    if dirty_nodes.is_empty() || tree.root.children.len() != fragments.root.children.len() {
        return None;
    }

    tree.root
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| {
            dirty_nodes
                .iter()
                .any(|dirty| layout_node_contains(child, *dirty))
        })
        .map(|(index, _)| index)
        .min()
}

pub fn relayout_fragment_flow(
    tree: &LayoutTree,
    fragments: &mut FragmentTree,
    dirty_nodes: &[NodeId],
) -> bool {
    let Some(start_index) = fragment_flow_start_index(tree, fragments, dirty_nodes) else {
        return false;
    };

    let containing_block = ContainingBlock {
        origin: fragments.root.boxes.content_box.origin,
        available: fragments.root.boxes.content_box.size,
    };
    let next_id = max_fragment_id(&fragments.root).saturating_add(1);
    let mut builder = FragmentBuilder {
        next_id,
        ..FragmentBuilder::default()
    };
    builder.prepare_margin_profiles(&tree.root);

    let (mut cursor_y, pending_margin) = preceding_flow_state(
        tree,
        fragments,
        start_index,
        containing_block.origin.y,
        &builder,
    );
    let mut retained_ids = std::collections::BTreeMap::new();
    for child in &fragments.root.children[start_index..] {
        collect_fragment_ids(child, &mut retained_ids);
    }

    let (mut rebuilt, _) = builder.layout_siblings(
        &tree.root.children[start_index..],
        containing_block,
        &mut cursor_y,
        pending_margin,
        false,
    );
    for child in &mut rebuilt {
        reuse_fragment_ids(child, &retained_ids);
    }

    fragments.root.children.truncate(start_index);
    fragments.root.children.extend(rebuilt);
    true
}

struct FragmentingInlineTextLeaf<'a> {
    boxes: Vec<&'a LayoutNode>,
    text_node: &'a LayoutNode,
    run: &'a TextRun,
}

struct FragmentingInlineTextStream<'a> {
    root: &'a LayoutNode,
    leaves: Vec<FragmentingInlineTextLeaf<'a>>,
}

fn fragmenting_inline_box_is_supported(node: &LayoutNode, nested: bool) -> bool {
    let style = node.style;
    style.display_inline
        && style.width.is_none()
        && style.height.is_none()
        && style.min_width.is_none()
        && style.max_width.is_none()
        && style.min_height.is_none()
        && style.max_height.is_none()
        && style.margin.top == 0.0
        && style.margin.bottom == 0.0
        && style.border_width.top == 0.0
        && style.border_width.bottom == 0.0
        && style.padding.top == 0.0
        && style.padding.bottom == 0.0
        && (!nested || style.vertical_align == VerticalAlign::Baseline)
}

fn collect_fragmenting_inline_text_leaves<'a>(
    node: &'a LayoutNode,
    boxes: &mut Vec<&'a LayoutNode>,
    leaves: &mut Vec<FragmentingInlineTextLeaf<'a>>,
) -> bool {
    if !fragmenting_inline_box_is_supported(node, true) || node.children.is_empty() {
        return false;
    }

    boxes.push(node);
    for child in &node.children {
        match &child.kind {
            LayoutNodeKind::Text(run) => leaves.push(FragmentingInlineTextLeaf {
                boxes: boxes.clone(),
                text_node: child,
                run,
            }),
            LayoutNodeKind::Box => {
                if !collect_fragmenting_inline_text_leaves(child, boxes, leaves) {
                    boxes.pop();
                    return false;
                }
            }
            LayoutNodeKind::Root => {
                boxes.pop();
                return false;
            }
        }
    }
    boxes.pop();
    true
}

fn fragmenting_inline_text_stream(node: &LayoutNode) -> Option<FragmentingInlineTextStream<'_>> {
    if !fragmenting_inline_box_is_supported(node, false) || node.children.is_empty() {
        return None;
    }

    let mut leaves = Vec::new();
    let mut boxes = Vec::new();
    for child in &node.children {
        match &child.kind {
            LayoutNodeKind::Text(run) => leaves.push(FragmentingInlineTextLeaf {
                boxes: Vec::new(),
                text_node: child,
                run,
            }),
            LayoutNodeKind::Box => {
                if !collect_fragmenting_inline_text_leaves(child, &mut boxes, &mut leaves) {
                    return None;
                }
            }
            LayoutNodeKind::Root => return None,
        }
    }

    (!leaves.is_empty()).then_some(FragmentingInlineTextStream { root: node, leaves })
}

fn is_inline_flow_node(node: &LayoutNode) -> bool {
    matches!(&node.kind, LayoutNodeKind::Text(_))
        || (matches!(&node.kind, LayoutNodeKind::Box) && node.style.display_inline)
}

fn preceding_flow_state(
    tree: &LayoutTree,
    fragments: &FragmentTree,
    start_index: usize,
    origin_y: f32,
    builder: &FragmentBuilder,
) -> (f32, MarginStrut) {
    if start_index == 0 {
        return (origin_y, MarginStrut::default());
    }

    let mut pending = MarginStrut::default();
    let mut index = start_index;
    while index > 0 {
        index -= 1;
        let node = &tree.root.children[index];
        let fragment = &fragments.root.children[index];
        if is_inline_flow_node(node) {
            let mut max_bottom =
                fragment.boxes.margin_box.origin.y + fragment.boxes.margin_box.size.height;
            while index > 0 {
                let previous_index = index - 1;
                let previous_node = &tree.root.children[previous_index];
                if !is_inline_flow_node(previous_node) {
                    break;
                }
                let previous_fragment = &fragments.root.children[previous_index];
                max_bottom = max_bottom.max(
                    previous_fragment.boxes.margin_box.origin.y
                        + previous_fragment.boxes.margin_box.size.height,
                );
                index = previous_index;
            }
            return (max_bottom, MarginStrut::default());
        }
        match node.kind {
            LayoutNodeKind::Box => {
                let profile = builder.margin_profile(node);
                if profile.through {
                    pending.adjoin(profile.before);
                    continue;
                }
                pending.adjoin(profile.after);
                return (
                    fragment.boxes.border_box.origin.y + fragment.boxes.border_box.size.height,
                    pending,
                );
            }
            LayoutNodeKind::Text(_) => unreachable!("inline flow text handled above"),
            LayoutNodeKind::Root => unreachable!("root cannot be its own child"),
        }
    }
    (origin_y, pending)
}

fn collect_margin_profiles(
    node: &LayoutNode,
    profiles: &mut std::collections::BTreeMap<LayoutNodeId, BlockMarginProfile>,
) {
    for child in &node.children {
        collect_margin_profiles(child, profiles);
    }
    if matches!(node.kind, LayoutNodeKind::Box) && !node.style.display_inline {
        let profile = block_margin_profile(node, profiles);
        profiles.insert(node.id, profile);
    }
}

fn block_margin_profile(
    node: &LayoutNode,
    profiles: &std::collections::BTreeMap<LayoutNodeId, BlockMarginProfile>,
) -> BlockMarginProfile {
    let style = node.style;
    let top_boundary =
        node.margin_collapse_boundary || style.border_width.top != 0.0 || style.padding.top != 0.0;
    let bottom_boundary = node.margin_collapse_boundary
        || style.border_width.bottom != 0.0
        || style.padding.bottom != 0.0
        || style.height.is_some()
        || style.min_height.is_some_and(|height| height > 0.0);

    let children_collapse_through = node.children.iter().all(|child| match child.kind {
        LayoutNodeKind::Box => profiles
            .get(&child.id)
            .is_some_and(|profile| profile.through),
        LayoutNodeKind::Text(_) | LayoutNodeKind::Root => false,
    });
    let through = !node.margin_collapse_boundary
        && style.border_width.vertical() == 0.0
        && style.padding.vertical() == 0.0
        && style.height.is_none_or(|height| height == 0.0)
        && style.min_height.is_none_or(|height| height == 0.0)
        && children_collapse_through;

    if through {
        let mut combined = MarginStrut::from_margin(style.margin.top);
        combined.adjoin_margin(style.margin.bottom);
        for child in &node.children {
            if let Some(profile) = profiles.get(&child.id) {
                combined.adjoin(profile.before);
                combined.adjoin(profile.after);
            }
        }
        return BlockMarginProfile {
            before: combined,
            after: combined,
            through: true,
            collapse_first_child: !node.children.is_empty(),
            collapse_last_child: !node.children.is_empty(),
        };
    }

    let mut before = MarginStrut::from_margin(style.margin.top);
    let mut collapse_first_child = false;
    if !top_boundary {
        for child in &node.children {
            let LayoutNodeKind::Box = child.kind else {
                break;
            };
            let Some(profile) = profiles.get(&child.id) else {
                break;
            };
            collapse_first_child = true;
            before.adjoin(profile.before);
            if profile.through {
                before.adjoin(profile.after);
            } else {
                break;
            }
        }
    }

    let mut after = MarginStrut::from_margin(style.margin.bottom);
    let mut collapse_last_child = false;
    if !bottom_boundary {
        for child in node.children.iter().rev() {
            let LayoutNodeKind::Box = child.kind else {
                break;
            };
            let Some(profile) = profiles.get(&child.id) else {
                break;
            };
            collapse_last_child = true;
            after.adjoin(profile.after);
            if profile.through {
                after.adjoin(profile.before);
            } else {
                break;
            }
        }
    }

    BlockMarginProfile {
        before,
        after,
        through: false,
        collapse_first_child,
        collapse_last_child,
    }
}

fn collect_fragment_ids(
    fragment: &Fragment,
    ids: &mut std::collections::BTreeMap<(LayoutNodeId, FragmentOrdinal), FragmentId>,
) {
    ids.insert((fragment.layout_node, fragment.ordinal), fragment.id);
    for child in &fragment.children {
        collect_fragment_ids(child, ids);
    }
}

fn reuse_fragment_ids(
    fragment: &mut Fragment,
    ids: &std::collections::BTreeMap<(LayoutNodeId, FragmentOrdinal), FragmentId>,
) {
    if let Some(id) = ids.get(&(fragment.layout_node, fragment.ordinal)) {
        fragment.id = *id;
    }
    for child in &mut fragment.children {
        reuse_fragment_ids(child, ids);
    }
}

fn layout_node_contains(node: &LayoutNode, dom_node: NodeId) -> bool {
    node.dom_node == Some(dom_node)
        || node
            .children
            .iter()
            .any(|child| layout_node_contains(child, dom_node))
}

fn find_layout_node(node: &LayoutNode, dom_node: NodeId) -> Option<&LayoutNode> {
    if node.dom_node == Some(dom_node) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_layout_node(child, dom_node))
}

fn find_fragment(fragment: &Fragment, dom_node: NodeId) -> Option<&Fragment> {
    if fragment.dom_node == Some(dom_node) {
        return Some(fragment);
    }
    fragment
        .children
        .iter()
        .find_map(|child| find_fragment(child, dom_node))
}

fn collect_fragments<'a>(fragment: &'a Fragment, dom_node: NodeId, output: &mut Vec<&'a Fragment>) {
    if fragment.dom_node == Some(dom_node) {
        output.push(fragment);
    }
    for child in &fragment.children {
        collect_fragments(child, dom_node, output);
    }
}

fn max_fragment_id(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .map(max_fragment_id)
        .fold(fragment.id.index(), usize::max)
}

fn relayout_fragment_child(
    parent: &mut Fragment,
    layout_node: &LayoutNode,
    dom_node: NodeId,
    builder: &mut FragmentBuilder,
) -> bool {
    let containing_block = ContainingBlock {
        origin: parent.boxes.content_box.origin,
        available: parent.boxes.content_box.size,
    };

    for child in &mut parent.children {
        if child.dom_node == Some(dom_node) {
            let mut cursor_y = child.boxes.border_box.origin.y;
            let mut replacement = builder.layout_node(layout_node, containing_block, &mut cursor_y);
            if replacement.len() != 1 {
                return false;
            }
            *child = replacement.remove(0);
            return true;
        }
        if relayout_fragment_child(child, layout_node, dom_node, builder) {
            return true;
        }
    }
    false
}

struct LayoutTreeBuilder<'a> {
    next_id: usize,
    styles: &'a StyleSet,
}

impl<'a> LayoutTreeBuilder<'a> {
    fn new(styles: &'a StyleSet) -> Self {
        Self { next_id: 0, styles }
    }

    fn build_node(
        &mut self,
        doc: &Document,
        node: NodeId,
        parent_style: Option<ComputedStyle>,
    ) -> Option<LayoutNode> {
        let dom_node = doc.node(node)?;
        let (kind, style) = match &dom_node.kind {
            NodeKind::Document => (LayoutNodeKind::Root, ComputedStyle::default()),
            NodeKind::Text(text) => (
                LayoutNodeKind::Text(TextRun::new(text.clone())),
                computed_style_with_parent(doc, node, self.styles, parent_style),
            ),
            NodeKind::Element(_) => {
                let style = computed_style_with_parent(doc, node, self.styles, parent_style);
                if style.display_none {
                    return None;
                }
                (LayoutNodeKind::Box, style)
            }
        };

        let margin_collapse_boundary = matches!(dom_node.kind, NodeKind::Document)
            || dom_node
                .parent
                .is_some_and(|parent| document_node_is_root(doc, parent))
            || style.establishes_bfc
            || style.display_flex
            || style.display_grid
            || parent_style.is_some_and(|parent| parent.display_flex || parent.display_grid);

        let id = self.allocate_id();
        let mut children = Vec::new();
        for child in doc.children(node).unwrap_or(&[]) {
            if let Some(layout_child) = self.build_node(doc, *child, Some(style)) {
                children.push(layout_child);
            }
        }

        let intrinsic = intrinsic_sizes_for_node(&kind, style, &children);

        Some(LayoutNode {
            id,
            dom_node: Some(node),
            kind,
            style,
            intrinsic,
            children,
            margin_collapse_boundary,
        })
    }

    fn allocate_id(&mut self) -> LayoutNodeId {
        let id = LayoutNodeId(self.next_id);
        self.next_id += 1;
        id
    }
}

fn document_node_is_root(document: &Document, node: NodeId) -> bool {
    document
        .node(node)
        .is_some_and(|node| matches!(node.kind, NodeKind::Document))
}

fn intrinsic_sizes_for_node(
    kind: &LayoutNodeKind,
    style: ComputedStyle,
    children: &[LayoutNode],
) -> IntrinsicSizes {
    match kind {
        LayoutNodeKind::Text(run) => run.intrinsic_sizes(),
        LayoutNodeKind::Root => IntrinsicSizes {
            min_content: children
                .iter()
                .map(|child| child.intrinsic.min_content)
                .fold(0.0, f32::max),
            max_content: children
                .iter()
                .map(|child| child.intrinsic.max_content)
                .fold(0.0, f32::max),
        },
        LayoutNodeKind::Box => {
            let horizontal_edges = style.padding.horizontal() + style.border_width.horizontal();
            let child_min = children
                .iter()
                .map(|child| child.intrinsic.min_content)
                .fold(0.0, f32::max);
            let child_max = children
                .iter()
                .map(|child| child.intrinsic.max_content)
                .fold(0.0, f32::max);
            if let Some(width) = style.width {
                let outer = clamp_used_dimension(width, style.min_width, style.max_width)
                    + horizontal_edges;
                IntrinsicSizes {
                    min_content: outer,
                    max_content: outer,
                }
            } else {
                IntrinsicSizes {
                    min_content: clamp_used_dimension(child_min, style.min_width, style.max_width)
                        + horizontal_edges,
                    max_content: clamp_used_dimension(child_max, style.min_width, style.max_width)
                        + horizontal_edges,
                }
            }
        }
    }
}

fn clamp_used_dimension(value: f32, minimum: Option<f32>, maximum: Option<f32>) -> f32 {
    let mut used = value.max(0.0);
    if let Some(maximum) = maximum {
        used = used.min(maximum.max(0.0));
    }
    if let Some(minimum) = minimum {
        used = used.max(minimum.max(0.0));
    }
    used
}

#[derive(Clone, Copy, Debug, Default)]
struct MarginStrut {
    positive: f32,
    negative: f32,
}

impl MarginStrut {
    fn from_margin(value: f32) -> Self {
        let mut strut = Self::default();
        strut.adjoin_margin(value);
        strut
    }

    fn adjoin_margin(&mut self, value: f32) {
        self.positive = self.positive.max(value.max(0.0));
        self.negative = self.negative.min(value.min(0.0));
    }

    fn adjoin(&mut self, other: Self) {
        self.positive = self.positive.max(other.positive);
        self.negative = self.negative.min(other.negative);
    }

    fn resolved(self) -> f32 {
        self.positive + self.negative
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct BlockMarginProfile {
    before: MarginStrut,
    after: MarginStrut,
    through: bool,
    collapse_first_child: bool,
    collapse_last_child: bool,
}

#[derive(Clone, Copy, Debug)]
struct InlineAlignmentItem {
    fragment_index: usize,
    vertical_align: VerticalAlign,
    baseline_offset: Option<f32>,
}

#[derive(Debug)]
struct InlineLineState {
    start_x: f32,
    right: f32,
    x: f32,
    active: bool,
    items: Vec<InlineAlignmentItem>,
}

impl InlineLineState {
    fn new(containing_block: ContainingBlock) -> Self {
        let start_x = containing_block.origin.x;
        Self {
            start_x,
            right: start_x + containing_block.available.width.max(0.0),
            x: start_x,
            active: false,
            items: Vec::new(),
        }
    }

    fn remaining_width(&self) -> f32 {
        (self.right - self.x).max(0.0)
    }

    fn record(
        &mut self,
        fragment_index: usize,
        vertical_align: VerticalAlign,
        baseline_offset: Option<f32>,
    ) {
        self.items.push(InlineAlignmentItem {
            fragment_index,
            vertical_align,
            baseline_offset,
        });
        self.active = true;
    }

    fn reset(&mut self) {
        self.x = self.start_x;
        self.active = false;
        self.items.clear();
    }
}

struct InlineTextContainerFlow<'a> {
    containing_block: ContainingBlock,
    cursor_y: &'a mut f32,
    line: &'a mut InlineLineState,
    fragments: &'a mut Vec<Fragment>,
}

struct InlineStreamPiece<'leaf, 'tree> {
    leaf: &'leaf FragmentingInlineTextLeaf<'tree>,
    owner_spans: &'leaf std::collections::BTreeMap<LayoutNodeId, (usize, usize)>,
    leaf_index: usize,
    leaf_ordinal: u32,
    text_range: TextRange,
    content_width: f32,
    is_first_leaf_piece: bool,
    is_last_leaf_piece: bool,
    cursor_y: f32,
}

#[derive(Default)]
struct FragmentBuilder {
    next_id: usize,
    margin_profiles: std::collections::BTreeMap<LayoutNodeId, BlockMarginProfile>,
}

impl FragmentBuilder {
    fn prepare_margin_profiles(&mut self, root: &LayoutNode) {
        self.margin_profiles.clear();
        collect_margin_profiles(root, &mut self.margin_profiles);
    }

    fn margin_profile(&self, node: &LayoutNode) -> BlockMarginProfile {
        self.margin_profiles
            .get(&node.id)
            .copied()
            .unwrap_or_default()
    }

    fn flush_inline_line(
        &self,
        line: &mut InlineLineState,
        fragments: &mut [Fragment],
        cursor_y: &mut f32,
    ) {
        if !line.active {
            return;
        }

        let mut baseline = 0.0f32;
        let mut descent = 0.0f32;
        let mut line_height = 0.0f32;
        for item in &line.items {
            let fragment = &fragments[item.fragment_index];
            let height = fragment.boxes.margin_box.size.height;
            line_height = line_height.max(height);
            if item.vertical_align == VerticalAlign::Baseline {
                let offset = item.baseline_offset.unwrap_or(height).clamp(0.0, height);
                baseline = baseline.max(offset);
                descent = descent.max((height - offset).max(0.0));
            }
        }
        line_height = line_height.max(baseline + descent);

        for item in &line.items {
            let fragment = &mut fragments[item.fragment_index];
            let height = fragment.boxes.margin_box.size.height;
            let target_y = match item.vertical_align {
                VerticalAlign::Baseline => {
                    let offset = item.baseline_offset.unwrap_or(height).clamp(0.0, height);
                    *cursor_y + baseline - offset
                }
                VerticalAlign::Top => *cursor_y,
                VerticalAlign::Bottom => *cursor_y + line_height - height,
            };
            let delta = target_y - fragment.boxes.margin_box.origin.y;
            translate_fragment_y(fragment, delta);
        }

        *cursor_y += line_height;
        line.reset();
    }

    fn build(&mut self, tree: &LayoutTree, viewport: Size) -> FragmentTree {
        self.prepare_margin_profiles(&tree.root);
        let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
        let containing_block = ContainingBlock {
            origin: Point { x: 0.0, y: 0.0 },
            available: Size {
                width: viewport.width.max(0.0),
                height: viewport.height.max(0.0),
            },
        };
        let mut cursor_y = containing_block.origin.y;
        let (children, _) = self.layout_siblings(
            &tree.root.children,
            containing_block,
            &mut cursor_y,
            MarginStrut::default(),
            false,
        );

        FragmentTree {
            root: Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(0),
                layout_node: tree.root.id,
                dom_node: tree.root.dom_node,
                kind: FragmentKind::Root,
                boxes: BoxModel::single(viewport_rect),
                style: tree.root.style,
                text_range: None,
                line_box: None,
                children,
            },
        }
    }

    fn layout_siblings(
        &mut self,
        nodes: &[LayoutNode],
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
        mut pending_margin: MarginStrut,
        mut suppress_leading_margin: bool,
    ) -> (Vec<Fragment>, MarginStrut) {
        let mut fragments = Vec::new();
        let mut line = InlineLineState::new(containing_block);

        for node in nodes {
            if matches!(&node.kind, LayoutNodeKind::Box) {
                if let Some(stream) = fragmenting_inline_text_stream(node) {
                    suppress_leading_margin = false;
                    if !line.active {
                        *cursor_y += pending_margin.resolved();
                        pending_margin = MarginStrut::default();
                    }
                    self.layout_inline_text_stream_flow(
                        stream,
                        InlineTextContainerFlow {
                            containing_block,
                            cursor_y,
                            line: &mut line,
                            fragments: &mut fragments,
                        },
                    );
                    continue;
                }
            }

            match &node.kind {
                LayoutNodeKind::Box if node.style.display_inline => {
                    suppress_leading_margin = false;
                    if !line.active {
                        *cursor_y += pending_margin.resolved();
                        pending_margin = MarginStrut::default();
                    }

                    let outer_width = self.inline_outer_width(node);
                    if line.active && line.x > line.start_x && line.x + outer_width > line.right {
                        self.flush_inline_line(&mut line, &mut fragments, cursor_y);
                    }

                    let fragment = self.layout_inline_box(
                        node,
                        containing_block,
                        Point {
                            x: line.x,
                            y: *cursor_y,
                        },
                    );
                    let width = fragment.boxes.margin_box.size.width;
                    let height = fragment.boxes.margin_box.size.height;
                    let fragment_index = fragments.len();
                    fragments.push(fragment);
                    line.x += width;
                    line.record(
                        fragment_index,
                        node.style.vertical_align,
                        (node.style.vertical_align == VerticalAlign::Baseline).then_some(height),
                    );
                }
                LayoutNodeKind::Box => {
                    self.flush_inline_line(&mut line, &mut fragments, cursor_y);

                    let profile = self.margin_profile(node);
                    if profile.through {
                        if !suppress_leading_margin {
                            pending_margin.adjoin(profile.before);
                        }
                        fragments.extend(self.layout_node(node, containing_block, cursor_y));
                        continue;
                    }

                    if suppress_leading_margin {
                        suppress_leading_margin = false;
                    } else {
                        pending_margin.adjoin(profile.before);
                        *cursor_y += pending_margin.resolved();
                    }
                    fragments.extend(self.layout_node(node, containing_block, cursor_y));
                    pending_margin = profile.after;
                }
                LayoutNodeKind::Text(run) => {
                    suppress_leading_margin = false;
                    if !line.active {
                        *cursor_y += pending_margin.resolved();
                        pending_margin = MarginStrut::default();
                    }
                    self.layout_text_inline_flow(
                        node,
                        run,
                        containing_block,
                        cursor_y,
                        &mut line,
                        &mut fragments,
                    );
                }
                LayoutNodeKind::Root => {
                    unreachable!("only the layout root may have Root kind")
                }
            }
        }

        self.flush_inline_line(&mut line, &mut fragments, cursor_y);
        (fragments, pending_margin)
    }

    fn layout_node(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Vec<Fragment> {
        match &node.kind {
            LayoutNodeKind::Root => unreachable!("only the layout root may have Root kind"),
            LayoutNodeKind::Text(run) => self.layout_text(node, run, containing_block, cursor_y),
            LayoutNodeKind::Box if node.style.display_grid => {
                vec![self.layout_grid_box(node, containing_block, cursor_y)]
            }
            LayoutNodeKind::Box if node.style.display_flex => {
                vec![self.layout_flex_box(node, containing_block, cursor_y)]
            }
            LayoutNodeKind::Box if node.style.display_inline => vec![self.layout_inline_box(
                node,
                containing_block,
                Point {
                    x: containing_block.origin.x,
                    y: *cursor_y,
                },
            )],
            LayoutNodeKind::Box => vec![self.layout_box(node, containing_block, cursor_y)],
        }
    }

    fn layout_inline_text_stream_flow(
        &mut self,
        stream: FragmentingInlineTextStream<'_>,
        flow: InlineTextContainerFlow<'_>,
    ) {
        let InlineTextContainerFlow {
            containing_block,
            cursor_y,
            line,
            fragments,
        } = flow;
        let root_style = stream.root.style;
        let root_left_edge = inline_left_edge(root_style);
        let root_right_edge = inline_right_edge(root_style);
        let full_width = containing_block.available.width.max(0.0);
        let leaf_count = stream.leaves.len();
        let mut root_ordinal = 0u32;
        let mut current_root_index = None;
        let mut owner_spans = std::collections::BTreeMap::<LayoutNodeId, (usize, usize)>::new();
        let mut owner_ordinals = std::collections::BTreeMap::<LayoutNodeId, u32>::new();

        for (leaf_index, leaf) in stream.leaves.iter().enumerate() {
            for owner in &leaf.boxes {
                owner_spans
                    .entry(owner.id)
                    .and_modify(|span| span.1 = leaf_index)
                    .or_insert((leaf_index, leaf_index));
            }
        }

        for (leaf_index, leaf) in stream.leaves.iter().enumerate() {
            let nested_left_edge = leaf
                .boxes
                .iter()
                .filter(|owner| {
                    owner_spans
                        .get(&owner.id)
                        .is_some_and(|span| span.0 == leaf_index)
                })
                .map(|owner| inline_left_edge(owner.style))
                .sum::<f32>();
            let nested_right_edge = leaf
                .boxes
                .iter()
                .filter(|owner| {
                    owner_spans
                        .get(&owner.id)
                        .is_some_and(|span| span.1 == leaf_index)
                })
                .map(|owner| inline_right_edge(owner.style))
                .sum::<f32>();
            let is_first_leaf = leaf_index == 0;
            let is_last_leaf = leaf_index + 1 == leaf_count;
            let first_piece_start_edge = nested_left_edge
                + if is_first_leaf && root_ordinal == 0 {
                    root_left_edge
                } else {
                    0.0
                };

            if line.active
                && leaf.run.shaped.clusters.first().is_some_and(|cluster| {
                    cluster.advance + first_piece_start_edge > line.remaining_width()
                })
            {
                let completed_root = current_root_index.take().is_some();
                self.flush_inline_line(line, fragments, cursor_y);
                if completed_root {
                    root_ordinal = root_ordinal.saturating_add(1);
                }
            }

            let first_width = (line.remaining_width() - first_piece_start_edge).max(0.0);
            let terminal_reserve =
                nested_right_edge + if is_last_leaf { root_right_edge } else { 0.0 };
            let ranges = UnicodeLineBreaker.break_text_with_widths_and_terminal_reserve(
                leaf.run,
                first_width,
                full_width,
                terminal_reserve,
            );
            let characters = leaf.run.text.chars().collect::<Vec<_>>();
            let range_count = ranges.len();

            for (leaf_ordinal, text_range) in ranges.into_iter().enumerate() {
                let is_first_leaf_piece = leaf_ordinal == 0;
                let is_last_leaf_piece = leaf_ordinal + 1 == range_count;
                let is_final_stream_piece = is_last_leaf && is_last_leaf_piece;

                let root_index = if let Some(index) = current_root_index {
                    index
                } else {
                    let mut style = root_style;
                    if root_ordinal != 0 {
                        style.margin.left = 0.0;
                        style.border_width.left = 0.0;
                        style.padding.left = 0.0;
                    }
                    style.margin.right = 0.0;
                    style.border_width.right = 0.0;
                    style.padding.right = 0.0;
                    let boxes = inline_fragment_box_model(
                        Point {
                            x: line.x,
                            y: *cursor_y,
                        },
                        style,
                        0.0,
                        leaf.run.line_height,
                    );
                    let index = fragments.len();
                    fragments.push(Fragment {
                        id: self.allocate_id(),
                        ordinal: FragmentOrdinal(root_ordinal),
                        layout_node: stream.root.id,
                        dom_node: stream.root.dom_node,
                        kind: FragmentKind::Box,
                        boxes,
                        style,
                        text_range: None,
                        line_box: None,
                        children: Vec::new(),
                    });
                    line.x = boxes.content_box.origin.x;
                    line.record(
                        index,
                        root_style.vertical_align,
                        Some(leaf.run.shaped.metrics.ascent),
                    );
                    current_root_index = Some(index);
                    index
                };

                let nested_start_reserve = if is_first_leaf_piece {
                    nested_left_edge
                } else {
                    0.0
                };
                let nested_terminal_reserve = if is_last_leaf_piece {
                    nested_right_edge
                } else {
                    0.0
                };
                let root_terminal_reserve = if is_final_stream_piece {
                    root_right_edge
                } else {
                    0.0
                };
                let available_content = (line.remaining_width()
                    - nested_start_reserve
                    - nested_terminal_reserve
                    - root_terminal_reserve)
                    .max(0.0);
                let content_width = leaf
                    .run
                    .advance_for_range(text_range)
                    .min(available_content);
                let piece = InlineStreamPiece {
                    leaf,
                    owner_spans: &owner_spans,
                    leaf_index,
                    leaf_ordinal: leaf_ordinal as u32,
                    text_range,
                    content_width,
                    is_first_leaf_piece,
                    is_last_leaf_piece,
                    cursor_y: *cursor_y,
                };
                {
                    let root_fragment = &mut fragments[root_index];
                    self.append_inline_stream_piece(
                        root_fragment,
                        &leaf.boxes,
                        &piece,
                        &mut owner_ordinals,
                        &mut line.x,
                    );
                    refresh_inline_fragment_from_children(root_fragment);
                }

                if is_final_stream_piece {
                    let root_fragment = &mut fragments[root_index];
                    root_fragment.style.margin.right = root_style.margin.right;
                    root_fragment.style.border_width.right = root_style.border_width.right;
                    root_fragment.style.padding.right = root_style.padding.right;
                    refresh_inline_fragment_from_children(root_fragment);
                    line.x += root_right_edge;
                }

                let mandatory = text_range
                    .end
                    .checked_sub(1)
                    .and_then(|index| characters.get(index))
                    .copied()
                    .is_some_and(is_mandatory_break);
                if mandatory || leaf_ordinal + 1 < range_count {
                    self.flush_inline_line(line, fragments, cursor_y);
                    current_root_index = None;
                    root_ordinal = root_ordinal.saturating_add(1);
                }
            }
        }
    }

    fn append_inline_stream_piece(
        &mut self,
        parent: &mut Fragment,
        owners: &[&LayoutNode],
        piece: &InlineStreamPiece<'_, '_>,
        owner_ordinals: &mut std::collections::BTreeMap<LayoutNodeId, u32>,
        line_x: &mut f32,
    ) {
        let Some((owner, remaining_owners)) = owners.split_first() else {
            let rect = Rect::new(
                *line_x,
                piece.cursor_y,
                piece.content_width,
                piece.leaf.run.line_height,
            );
            parent.children.push(Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(piece.leaf_ordinal),
                layout_node: piece.leaf.text_node.id,
                dom_node: piece.leaf.text_node.dom_node,
                kind: FragmentKind::Text,
                boxes: BoxModel::single(rect),
                style: piece.leaf.text_node.style,
                text_range: Some(piece.text_range),
                line_box: Some(LineBox {
                    ordinal: piece.leaf_ordinal,
                    rect,
                    text_range: piece.text_range,
                }),
                children: Vec::new(),
            });
            *line_x += piece.content_width;
            return;
        };

        let span = piece
            .owner_spans
            .get(&owner.id)
            .copied()
            .expect("owner span exists for every inline leaf owner");
        let is_owner_first_piece = piece.is_first_leaf_piece && piece.leaf_index == span.0;
        let is_owner_last_piece = piece.is_last_leaf_piece && piece.leaf_index == span.1;
        let reuses_last = parent.children.last().is_some_and(|fragment| {
            fragment.kind == FragmentKind::Box && fragment.layout_node == owner.id
        });

        if !reuses_last {
            let ordinal = owner_ordinals.entry(owner.id).or_insert(0);
            let fragment_ordinal = *ordinal;
            *ordinal = ordinal.saturating_add(1);
            let mut style = owner.style;
            if !is_owner_first_piece {
                style.margin.left = 0.0;
                style.border_width.left = 0.0;
                style.padding.left = 0.0;
            }
            style.margin.right = 0.0;
            style.border_width.right = 0.0;
            style.padding.right = 0.0;
            let boxes = inline_fragment_box_model(
                Point {
                    x: *line_x,
                    y: piece.cursor_y,
                },
                style,
                0.0,
                piece.leaf.run.line_height,
            );
            *line_x = boxes.content_box.origin.x;
            parent.children.push(Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(fragment_ordinal),
                layout_node: owner.id,
                dom_node: owner.dom_node,
                kind: FragmentKind::Box,
                boxes,
                style,
                text_range: None,
                line_box: None,
                children: Vec::new(),
            });
        }

        let owner_fragment = parent
            .children
            .last_mut()
            .expect("inline owner fragment was created or reused");
        self.append_inline_stream_piece(
            owner_fragment,
            remaining_owners,
            piece,
            owner_ordinals,
            line_x,
        );

        if is_owner_last_piece {
            owner_fragment.style.margin.right = owner.style.margin.right;
            owner_fragment.style.border_width.right = owner.style.border_width.right;
            owner_fragment.style.padding.right = owner.style.padding.right;
            *line_x += inline_right_edge(owner.style);
        }
        refresh_inline_fragment_from_children(owner_fragment);
    }

    fn layout_text_inline_flow(
        &mut self,
        node: &LayoutNode,
        run: &TextRun,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
        line: &mut InlineLineState,
        fragments: &mut Vec<Fragment>,
    ) {
        let full_width = containing_block.available.width.max(0.0);
        if line.active
            && run
                .shaped
                .clusters
                .first()
                .is_some_and(|cluster| cluster.advance > line.remaining_width())
        {
            self.flush_inline_line(line, fragments, cursor_y);
        }

        let first_width = if line.active {
            line.remaining_width()
        } else {
            full_width
        };
        let ranges = UnicodeLineBreaker.break_text_with_widths(run, first_width, full_width);
        let characters = run.text.chars().collect::<Vec<_>>();
        let range_count = ranges.len();

        for (ordinal, text_range) in ranges.into_iter().enumerate() {
            let available_width = line.remaining_width();
            let width = run.advance_for_range(text_range).min(available_width);
            let rect = Rect::new(line.x, *cursor_y, width, run.line_height);
            let line_box = LineBox {
                ordinal: ordinal as u32,
                rect,
                text_range,
            };
            let fragment_index = fragments.len();
            fragments.push(Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(ordinal as u32),
                layout_node: node.id,
                dom_node: node.dom_node,
                kind: FragmentKind::Text,
                boxes: BoxModel::single(rect),
                style: node.style,
                text_range: Some(text_range),
                line_box: Some(line_box),
                children: Vec::new(),
            });

            line.x += width;
            line.record(
                fragment_index,
                VerticalAlign::Baseline,
                Some(run.shaped.metrics.ascent),
            );
            let mandatory = text_range
                .end
                .checked_sub(1)
                .and_then(|index| characters.get(index))
                .copied()
                .is_some_and(is_mandatory_break);
            if mandatory || ordinal + 1 < range_count {
                self.flush_inline_line(line, fragments, cursor_y);
            }
        }
    }

    fn layout_text(
        &mut self,
        node: &LayoutNode,
        run: &TextRun,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Vec<Fragment> {
        let available_width = containing_block.available.width.max(0.0);
        let line_breaker = UnicodeLineBreaker;
        let ranges = line_breaker.break_text(run, available_width);
        let mut fragments = Vec::with_capacity(ranges.len());
        for (ordinal, text_range) in ranges.into_iter().enumerate() {
            let width = run.advance_for_range(text_range).min(available_width);
            let rect = Rect::new(containing_block.origin.x, *cursor_y, width, run.line_height);
            *cursor_y += run.line_height;
            let line_box = LineBox {
                ordinal: ordinal as u32,
                rect,
                text_range,
            };
            fragments.push(Fragment {
                id: self.allocate_id(),
                ordinal: FragmentOrdinal(ordinal as u32),
                layout_node: node.id,
                dom_node: node.dom_node,
                kind: FragmentKind::Text,
                boxes: BoxModel::single(rect),
                style: node.style,
                text_range: Some(text_range),
                line_box: Some(line_box),
                children: Vec::new(),
            });
        }
        fragments
    }

    fn inline_content_width(&self, node: &LayoutNode) -> f32 {
        let style = node.style;
        let non_content = style.padding.horizontal() + style.border_width.horizontal();
        let intrinsic_content = (node.intrinsic.max_content - non_content).max(0.0);
        clamp_used_dimension(
            style.width.unwrap_or(intrinsic_content),
            style.min_width,
            style.max_width,
        )
    }

    fn inline_outer_width(&self, node: &LayoutNode) -> f32 {
        let style = node.style;
        self.inline_content_width(node)
            + style.padding.horizontal()
            + style.border_width.horizontal()
            + style.margin.horizontal()
    }

    fn layout_inline_box(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        origin: Point,
    ) -> Fragment {
        let style = node.style;
        let content_width = self.inline_content_width(node);
        let border_x = origin.x + style.margin.left;
        let border_y = origin.y + style.margin.top;
        let padding_x = border_x + style.border_width.left;
        let padding_y = border_y + style.border_width.top;
        let content_x = padding_x + style.padding.left;
        let content_y = padding_y + style.padding.top;

        let child_containing_block = ContainingBlock {
            origin: Point {
                x: content_x,
                y: content_y,
            },
            available: Size {
                width: content_width,
                height: containing_block.available.height,
            },
        };
        let mut child_y = child_containing_block.origin.y;
        let (children, trailing_margin) = self.layout_siblings(
            &node.children,
            child_containing_block,
            &mut child_y,
            MarginStrut::default(),
            false,
        );
        child_y += trailing_margin.resolved();

        let natural_content_height = (child_y - content_y).max(0.0);
        let content_height = clamp_used_dimension(
            style.height.unwrap_or(natural_content_height),
            style.min_height,
            style.max_height,
        );
        let content_box = Rect::new(content_x, content_y, content_width, content_height);
        let padding_box = Rect::new(
            padding_x,
            padding_y,
            content_width + style.padding.horizontal(),
            content_height + style.padding.vertical(),
        );
        let border_box = Rect::new(
            border_x,
            border_y,
            padding_box.size.width + style.border_width.horizontal(),
            padding_box.size.height + style.border_width.vertical(),
        );
        let margin_box = Rect::new(
            origin.x,
            origin.y,
            border_box.size.width + style.margin.horizontal(),
            border_box.size.height + style.margin.vertical(),
        );

        Fragment {
            id: self.allocate_id(),
            ordinal: FragmentOrdinal(0),
            layout_node: node.id,
            dom_node: node.dom_node,
            kind: FragmentKind::Box,
            boxes: BoxModel {
                margin_box,
                border_box,
                padding_box,
                content_box,
            },
            style,
            text_range: None,
            line_box: None,
            children,
        }
    }

    fn layout_grid_box(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Fragment {
        self.layout_grid_box_with_content_size(node, containing_block, cursor_y, None, None)
    }

    fn layout_grid_box_with_content_size(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
        content_width_override: Option<f32>,
        content_height_override: Option<f32>,
    ) -> Fragment {
        let style = node.style;
        let x = containing_block.origin.x;
        let available_width = containing_block.available.width;
        let horizontal_edges = style.margin.horizontal()
            + style.border_width.horizontal()
            + style.padding.horizontal();

        let content_width = content_width_override.unwrap_or_else(|| {
            clamp_used_dimension(
                style
                    .width
                    .unwrap_or_else(|| (available_width - horizontal_edges).max(0.0)),
                style.min_width,
                style.max_width,
            )
        });

        let border_x = x + style.margin.left;
        let border_y = *cursor_y;
        let margin_top = border_y - style.margin.top;
        let padding_x = border_x + style.border_width.left;
        let padding_y = border_y + style.border_width.top;
        let content_x = padding_x + style.padding.left;
        let content_y = padding_y + style.padding.top;
        let definite_content_height = content_height_override.or_else(|| {
            style
                .height
                .map(|height| clamp_used_dimension(height, style.min_height, style.max_height))
        });
        let available_content_height =
            definite_content_height.unwrap_or_else(|| containing_block.available.height.max(0.0));

        let child_containing_block = ContainingBlock {
            origin: Point {
                x: content_x,
                y: content_y,
            },
            available: Size {
                width: content_width,
                height: available_content_height,
            },
        };
        let (children, natural_content_size) =
            self.layout_grid_children(node, child_containing_block);
        let content_height = content_height_override.unwrap_or_else(|| {
            clamp_used_dimension(
                style.height.unwrap_or(natural_content_size.height),
                style.min_height,
                style.max_height,
            )
        });

        let content_box = Rect::new(content_x, content_y, content_width, content_height);
        let padding_box = Rect::new(
            padding_x,
            padding_y,
            content_width + style.padding.horizontal(),
            content_height + style.padding.vertical(),
        );
        let border_box = Rect::new(
            border_x,
            border_y,
            padding_box.size.width + style.border_width.horizontal(),
            padding_box.size.height + style.border_width.vertical(),
        );
        let margin_box = Rect::new(
            x,
            margin_top,
            border_box.size.width + style.margin.horizontal(),
            border_box.size.height + style.margin.vertical(),
        );

        *cursor_y = border_box.origin.y + border_box.size.height;

        Fragment {
            id: self.allocate_id(),
            ordinal: FragmentOrdinal(0),
            layout_node: node.id,
            dom_node: node.dom_node,
            kind: FragmentKind::Box,
            boxes: BoxModel {
                margin_box,
                border_box,
                padding_box,
                content_box,
            },
            style,
            text_range: None,
            line_box: None,
            children,
        }
    }

    fn layout_grid_children(
        &mut self,
        container: &LayoutNode,
        containing_block: ContainingBlock,
    ) -> (Vec<Fragment>, Size) {
        let columns = container
            .style
            .grid_template_columns
            .as_slice()
            .iter()
            .copied()
            .map(GridTrack::new)
            .collect::<Vec<_>>();
        let rows = container
            .style
            .grid_template_rows
            .as_slice()
            .iter()
            .copied()
            .map(GridTrack::new)
            .collect::<Vec<_>>();

        let Ok(explicit_grid) = layout_fixed_grid(
            containing_block.origin,
            containing_block.available,
            &columns,
            &rows,
            container.style.column_gap,
            container.style.row_gap,
            &[],
        ) else {
            return (Vec::new(), Size::default());
        };
        let explicit_content_size = explicit_grid.content_size;

        let mut nodes = Vec::new();
        let mut requests = Vec::new();
        for child in &container.children {
            match &child.kind {
                LayoutNodeKind::Text(run) if run.text.chars().all(char::is_whitespace) => continue,
                LayoutNodeKind::Box => {
                    requests.push(
                        GridPlacementRequest::auto(child.id)
                            .with_row_start(
                                child
                                    .style
                                    .grid_row_start
                                    .map(|start| usize::from(start) - 1),
                            )
                            .with_column_start(
                                child
                                    .style
                                    .grid_column_start
                                    .map(|start| usize::from(start) - 1),
                            )
                            .with_span(
                                usize::from(child.style.grid_row_span),
                                usize::from(child.style.grid_column_span),
                            ),
                    );
                    nodes.push(child);
                }
                LayoutNodeKind::Text(_) | LayoutNodeKind::Root => {
                    return (Vec::new(), explicit_content_size);
                }
            }
        }

        let Ok(layout) = layout_fixed_grid_with_auto_placement(
            containing_block.origin,
            containing_block.available,
            &columns,
            &rows,
            container.style.column_gap,
            container.style.row_gap,
            &requests,
        ) else {
            return (Vec::new(), explicit_content_size);
        };

        let mut fragments = Vec::with_capacity(nodes.len());
        for (child, placement) in nodes.into_iter().zip(&layout.items) {
            let style = child.style;
            let horizontal_noncontent =
                style.padding.horizontal() + style.border_width.horizontal();
            let vertical_noncontent = style.padding.vertical() + style.border_width.vertical();
            let stretch_content_width =
                (placement.area.size.width - style.margin.horizontal() - horizontal_noncontent)
                    .max(0.0);
            let stretch_content_height =
                (placement.area.size.height - style.margin.vertical() - vertical_noncontent)
                    .max(0.0);
            let content_width = style
                .width
                .map(|width| clamp_used_dimension(width, style.min_width, style.max_width))
                .unwrap_or_else(|| {
                    clamp_used_dimension(stretch_content_width, style.min_width, style.max_width)
                });
            let content_height = style
                .height
                .map(|height| clamp_used_dimension(height, style.min_height, style.max_height))
                .unwrap_or_else(|| {
                    clamp_used_dimension(stretch_content_height, style.min_height, style.max_height)
                });

            let child_containing_block = ContainingBlock {
                origin: placement.area.origin,
                available: placement.area.size,
            };
            let mut child_y = placement.area.origin.y + style.margin.top;
            let fragment = if style.display_grid {
                self.layout_grid_box_with_content_size(
                    child,
                    child_containing_block,
                    &mut child_y,
                    Some(content_width),
                    Some(content_height),
                )
            } else if style.display_flex {
                self.layout_flex_box_with_content_size(
                    child,
                    child_containing_block,
                    &mut child_y,
                    Some(content_width),
                    Some(content_height),
                )
            } else {
                self.layout_box_with_content_size(
                    child,
                    child_containing_block,
                    &mut child_y,
                    Some(content_width),
                    Some(content_height),
                )
            };
            fragments.push(fragment);
        }

        (fragments, layout.content_size)
    }

    fn layout_flex_box(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Fragment {
        self.layout_flex_box_with_content_size(node, containing_block, cursor_y, None, None)
    }

    fn layout_flex_box_with_content_size(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
        content_width_override: Option<f32>,
        content_height_override: Option<f32>,
    ) -> Fragment {
        let style = node.style;
        let x = containing_block.origin.x;
        let available_width = containing_block.available.width;
        let horizontal_edges = style.margin.horizontal()
            + style.border_width.horizontal()
            + style.padding.horizontal();

        let content_width = content_width_override.unwrap_or_else(|| {
            clamp_used_dimension(
                style
                    .width
                    .unwrap_or_else(|| (available_width - horizontal_edges).max(0.0)),
                style.min_width,
                style.max_width,
            )
        });

        let border_x = x + style.margin.left;
        let border_y = *cursor_y;
        let margin_top = border_y - style.margin.top;
        let padding_x = border_x + style.border_width.left;
        let padding_y = border_y + style.border_width.top;
        let content_x = padding_x + style.padding.left;
        let content_y = padding_y + style.padding.top;
        let definite_content_height = content_height_override.or_else(|| {
            style
                .height
                .map(|height| clamp_used_dimension(height, style.min_height, style.max_height))
        });
        let available_content_height =
            definite_content_height.unwrap_or_else(|| containing_block.available.height.max(0.0));

        let child_containing_block = ContainingBlock {
            origin: Point {
                x: content_x,
                y: content_y,
            },
            available: Size {
                width: content_width,
                height: available_content_height,
            },
        };
        let (children, natural_content_height) =
            self.layout_flex_children(node, child_containing_block, definite_content_height);
        let content_height = content_height_override.unwrap_or_else(|| {
            clamp_used_dimension(
                style.height.unwrap_or(natural_content_height),
                style.min_height,
                style.max_height,
            )
        });

        let content_box = Rect::new(content_x, content_y, content_width, content_height);
        let padding_box = Rect::new(
            padding_x,
            padding_y,
            content_width + style.padding.horizontal(),
            content_height + style.padding.vertical(),
        );
        let border_box = Rect::new(
            border_x,
            border_y,
            padding_box.size.width + style.border_width.horizontal(),
            padding_box.size.height + style.border_width.vertical(),
        );
        let margin_box = Rect::new(
            x,
            margin_top,
            border_box.size.width + style.margin.horizontal(),
            border_box.size.height + style.margin.vertical(),
        );

        *cursor_y = border_box.origin.y + border_box.size.height;

        Fragment {
            id: self.allocate_id(),
            ordinal: FragmentOrdinal(0),
            layout_node: node.id,
            dom_node: node.dom_node,
            kind: FragmentKind::Box,
            boxes: BoxModel {
                margin_box,
                border_box,
                padding_box,
                content_box,
            },
            style,
            text_range: None,
            line_box: None,
            children,
        }
    }

    fn measure_flex_item_natural_border_height(
        &self,
        child: &LayoutNode,
        available_height: f32,
        resolved_border_width: f32,
    ) -> Option<f32> {
        let mut builder = FragmentBuilder {
            next_id: 0,
            margin_profiles: self.margin_profiles.clone(),
        };
        let containing_block = ContainingBlock {
            origin: Point { x: 0.0, y: 0.0 },
            available: Size {
                width: resolved_border_width + child.style.margin.horizontal(),
                height: available_height,
            },
        };
        let content_width = (resolved_border_width
            - child.style.padding.horizontal()
            - child.style.border_width.horizontal())
        .max(0.0);
        let mut cursor_y = child.style.margin.top;
        let fragment = if child.style.display_grid {
            builder.layout_grid_box_with_content_size(
                child,
                containing_block,
                &mut cursor_y,
                Some(content_width),
                None,
            )
        } else if child.style.display_flex {
            builder.layout_flex_box_with_content_size(
                child,
                containing_block,
                &mut cursor_y,
                Some(content_width),
                None,
            )
        } else {
            builder.layout_box_with_content_size(
                child,
                containing_block,
                &mut cursor_y,
                Some(content_width),
                None,
            )
        };
        fragment
            .boxes
            .border_box
            .size
            .height
            .is_finite()
            .then_some(fragment.boxes.border_box.size.height)
    }

    fn layout_flex_children(
        &mut self,
        container: &LayoutNode,
        containing_block: ContainingBlock,
        definite_cross_size: Option<f32>,
    ) -> (Vec<Fragment>, f32) {
        let mut nodes = Vec::new();
        let mut items = Vec::new();
        let mut item_cross_alignments = Vec::new();
        let mut item_cross_metadata = Vec::new();
        let container_cross_alignment = flex_cross_alignment(container.style.align_items);

        for child in &container.children {
            match &child.kind {
                LayoutNodeKind::Text(run) if run.text.chars().all(char::is_whitespace) => continue,
                LayoutNodeKind::Box => {
                    let style = child.style;
                    let Some(width) = style.width else {
                        return (Vec::new(), 0.0);
                    };
                    let content_width =
                        clamp_used_dimension(width, style.min_width, style.max_width);
                    let item_cross_alignment = flex_item_cross_alignment(style.align_self);
                    let effective_cross_alignment =
                        item_cross_alignment.unwrap_or(container_cross_alignment);
                    let content_height = if let Some(height) = style.height {
                        clamp_used_dimension(height, style.min_height, style.max_height)
                    } else if container.style.flex_wrap != FlexWrap::NoWrap {
                        clamp_used_dimension(0.0, style.min_height, style.max_height)
                    } else {
                        if effective_cross_alignment != FlexCrossAlignment::Stretch {
                            return (Vec::new(), 0.0);
                        }
                        let Some(definite_cross_size) = definite_cross_size else {
                            return (Vec::new(), 0.0);
                        };
                        stretched_flex_item_content_height(style, definite_cross_size)
                    };
                    let horizontal_noncontent =
                        style.padding.horizontal() + style.border_width.horizontal();
                    let vertical_noncontent =
                        style.padding.vertical() + style.border_width.vertical();
                    let effective_min_width = style.min_width.unwrap_or(0.0);
                    let effective_max_width = style
                        .max_width
                        .map(|maximum| maximum.max(effective_min_width));
                    items.push(
                        FlexibleFlexRowItem::new(
                            FlexRowItem::new(
                                child.id,
                                Size {
                                    width: content_width + horizontal_noncontent,
                                    height: content_height + vertical_noncontent,
                                },
                                style.margin,
                            ),
                            style.flex_grow,
                            style.flex_shrink,
                        )
                        .with_main_size_limits(
                            effective_min_width + horizontal_noncontent,
                            effective_max_width.map(|maximum| maximum + horizontal_noncontent),
                        ),
                    );
                    item_cross_alignments.push(item_cross_alignment);
                    let effective_min_height = style.min_height.unwrap_or(0.0);
                    let effective_max_height = style
                        .max_height
                        .map(|maximum| maximum.max(effective_min_height));
                    item_cross_metadata.push(if style.height.is_none() {
                        FlexCrossSizeMetadata::auto(
                            effective_min_height + vertical_noncontent,
                            effective_max_height.map(|maximum| maximum + vertical_noncontent),
                        )
                    } else {
                        FlexCrossSizeMetadata::default()
                    });
                    nodes.push(child);
                }
                LayoutNodeKind::Text(_) | LayoutNodeKind::Root => {
                    return (Vec::new(), 0.0);
                }
            }
        }

        let options = FlexRowOptions::default()
            .with_main_alignment(flex_main_alignment(container.style.justify_content))
            .with_main_reverse(container.style.flex_direction == FlexDirection::RowReverse)
            .with_cross_alignment(container_cross_alignment)
            .with_content_alignment(flex_content_alignment(container.style.align_content))
            .with_cross_reverse(container.style.flex_wrap == FlexWrap::WrapReverse)
            .with_main_gap(container.style.column_gap)
            .with_cross_gap(container.style.row_gap)
            .with_cross_size(definite_cross_size)
            .with_cross_size_limits(container.style.min_height, container.style.max_height);

        if container.style.flex_wrap != FlexWrap::NoWrap
            && item_cross_metadata.iter().any(|metadata| metadata.auto)
        {
            let Ok(provisional) = layout_wrapped_flexible_rows_with_item_alignments(
                containing_block.origin,
                containing_block.available,
                &items,
                options,
                &item_cross_alignments,
            ) else {
                return (Vec::new(), 0.0);
            };
            for (index, (child, placement)) in nodes.iter().zip(&provisional.items).enumerate() {
                if !item_cross_metadata[index].auto {
                    continue;
                }
                let Some(border_height) = self.measure_flex_item_natural_border_height(
                    child,
                    containing_block.available.height,
                    placement.border_box.size.width,
                ) else {
                    return (Vec::new(), 0.0);
                };
                items[index].item.base_size.height = border_height;
            }
        }

        let (placements, natural_content_height) = match container.style.flex_wrap {
            FlexWrap::NoWrap => {
                let Ok(row) = layout_flexible_single_line_flex_row_with_item_alignments(
                    containing_block.origin,
                    containing_block.available,
                    &items,
                    options,
                    &item_cross_alignments,
                ) else {
                    return (Vec::new(), 0.0);
                };
                (row.items, row.content_size.height)
            }
            FlexWrap::Wrap | FlexWrap::WrapReverse => {
                let Ok(layout) = layout_wrapped_flexible_rows_with_cross_metadata(
                    containing_block.origin,
                    containing_block.available,
                    &items,
                    options,
                    &item_cross_alignments,
                    &item_cross_metadata,
                ) else {
                    return (Vec::new(), 0.0);
                };
                (layout.items, layout.content_size.height)
            }
        };

        let mut fragments = Vec::with_capacity(nodes.len());
        for (child, placement) in nodes.into_iter().zip(placements.iter()) {
            let child_containing_block = ContainingBlock {
                origin: Point {
                    x: placement.border_box.origin.x - child.style.margin.left,
                    y: placement.border_box.origin.y - child.style.margin.top,
                },
                available: Size {
                    width: placement.border_box.size.width + child.style.margin.horizontal(),
                    height: containing_block.available.height,
                },
            };
            let flex_content_width = (placement.border_box.size.width
                - child.style.padding.horizontal()
                - child.style.border_width.horizontal())
            .max(0.0);
            let content_height_override = child.style.height.is_none().then(|| {
                (placement.border_box.size.height
                    - child.style.padding.vertical()
                    - child.style.border_width.vertical())
                .max(0.0)
            });
            let mut child_y = placement.border_box.origin.y;
            let fragment = if child.style.display_grid {
                self.layout_grid_box_with_content_size(
                    child,
                    child_containing_block,
                    &mut child_y,
                    Some(flex_content_width),
                    content_height_override,
                )
            } else if child.style.display_flex {
                self.layout_flex_box_with_content_size(
                    child,
                    child_containing_block,
                    &mut child_y,
                    Some(flex_content_width),
                    content_height_override,
                )
            } else {
                self.layout_box_with_content_size(
                    child,
                    child_containing_block,
                    &mut child_y,
                    Some(flex_content_width),
                    content_height_override,
                )
            };
            fragments.push(fragment);
        }

        (fragments, natural_content_height)
    }

    fn layout_box(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
    ) -> Fragment {
        self.layout_box_with_content_size(node, containing_block, cursor_y, None, None)
    }

    fn layout_box_with_content_size(
        &mut self,
        node: &LayoutNode,
        containing_block: ContainingBlock,
        cursor_y: &mut f32,
        content_width_override: Option<f32>,
        content_height_override: Option<f32>,
    ) -> Fragment {
        let style = node.style;
        let x = containing_block.origin.x;
        let available_width = containing_block.available.width;
        let horizontal_edges = style.margin.horizontal()
            + style.border_width.horizontal()
            + style.padding.horizontal();

        let content_width = content_width_override.unwrap_or_else(|| {
            clamp_used_dimension(
                style
                    .width
                    .unwrap_or_else(|| (available_width - horizontal_edges).max(0.0)),
                style.min_width,
                style.max_width,
            )
        });

        let border_x = x + style.margin.left;
        let border_y = *cursor_y;
        let margin_top = border_y - style.margin.top;
        let padding_x = border_x + style.border_width.left;
        let padding_y = border_y + style.border_width.top;
        let content_x = padding_x + style.padding.left;
        let content_y = padding_y + style.padding.top;

        let child_containing_block = ContainingBlock {
            origin: Point {
                x: content_x,
                y: content_y,
            },
            available: Size {
                width: content_width,
                height: content_height_override.unwrap_or(containing_block.available.height),
            },
        };
        let profile = self.margin_profile(node);
        let mut child_y = child_containing_block.origin.y;
        let (children, trailing_margin) = self.layout_siblings(
            &node.children,
            child_containing_block,
            &mut child_y,
            MarginStrut::default(),
            profile.collapse_first_child,
        );
        if !profile.collapse_last_child {
            child_y += trailing_margin.resolved();
        }

        let natural_content_height = (child_y - content_y).max(0.0);
        let content_height = content_height_override.unwrap_or_else(|| {
            clamp_used_dimension(
                style.height.unwrap_or(natural_content_height),
                style.min_height,
                style.max_height,
            )
        });

        let content_box = Rect::new(content_x, content_y, content_width, content_height);
        let padding_box = Rect::new(
            padding_x,
            padding_y,
            content_width + style.padding.horizontal(),
            content_height + style.padding.vertical(),
        );
        let border_box = Rect::new(
            border_x,
            border_y,
            padding_box.size.width + style.border_width.horizontal(),
            padding_box.size.height + style.border_width.vertical(),
        );
        let margin_box = Rect::new(
            x,
            margin_top,
            border_box.size.width + style.margin.horizontal(),
            border_box.size.height + style.margin.vertical(),
        );

        *cursor_y = border_box.origin.y + border_box.size.height;

        Fragment {
            id: self.allocate_id(),
            ordinal: FragmentOrdinal(0),
            layout_node: node.id,
            dom_node: node.dom_node,
            kind: FragmentKind::Box,
            boxes: BoxModel {
                margin_box,
                border_box,
                padding_box,
                content_box,
            },
            style,
            text_range: None,
            line_box: None,
            children,
        }
    }

    fn allocate_id(&mut self) -> FragmentId {
        let id = FragmentId(self.next_id);
        self.next_id += 1;
        id
    }
}

fn stretched_flex_item_content_height(style: ComputedStyle, line_cross_size: f32) -> f32 {
    let outer_noncontent =
        style.margin.vertical() + style.border_width.vertical() + style.padding.vertical();
    clamp_used_dimension(
        (line_cross_size - outer_noncontent).max(0.0),
        style.min_height,
        style.max_height,
    )
}

fn flex_item_cross_alignment(align_self: AlignSelf) -> Option<FlexCrossAlignment> {
    match align_self {
        AlignSelf::Auto => None,
        AlignSelf::Stretch => Some(FlexCrossAlignment::Stretch),
        AlignSelf::FlexStart => Some(FlexCrossAlignment::Start),
        AlignSelf::FlexEnd => Some(FlexCrossAlignment::End),
        AlignSelf::Center => Some(FlexCrossAlignment::Center),
    }
}

fn flex_content_alignment(align_content: AlignContent) -> FlexContentAlignment {
    match align_content {
        AlignContent::Stretch => FlexContentAlignment::Stretch,
        AlignContent::FlexStart => FlexContentAlignment::Start,
        AlignContent::FlexEnd => FlexContentAlignment::End,
        AlignContent::Center => FlexContentAlignment::Center,
        AlignContent::SpaceBetween => FlexContentAlignment::SpaceBetween,
        AlignContent::SpaceAround => FlexContentAlignment::SpaceAround,
        AlignContent::SpaceEvenly => FlexContentAlignment::SpaceEvenly,
    }
}

fn flex_cross_alignment(align_items: AlignItems) -> FlexCrossAlignment {
    match align_items {
        AlignItems::Stretch => FlexCrossAlignment::Stretch,
        AlignItems::FlexStart => FlexCrossAlignment::Start,
        AlignItems::FlexEnd => FlexCrossAlignment::End,
        AlignItems::Center => FlexCrossAlignment::Center,
    }
}

fn flex_main_alignment(justify_content: JustifyContent) -> FlexMainAlignment {
    match justify_content {
        JustifyContent::FlexStart => FlexMainAlignment::Start,
        JustifyContent::FlexEnd => FlexMainAlignment::End,
        JustifyContent::Center => FlexMainAlignment::Center,
        JustifyContent::SpaceBetween => FlexMainAlignment::SpaceBetween,
        JustifyContent::SpaceAround => FlexMainAlignment::SpaceAround,
        JustifyContent::SpaceEvenly => FlexMainAlignment::SpaceEvenly,
    }
}

fn inline_left_edge(style: ComputedStyle) -> f32 {
    style.margin.left + style.border_width.left + style.padding.left
}

fn inline_right_edge(style: ComputedStyle) -> f32 {
    style.padding.right + style.border_width.right + style.margin.right
}

fn refresh_inline_fragment_from_children(fragment: &mut Fragment) {
    let Some(first_child) = fragment.children.first() else {
        return;
    };
    let content_origin = fragment.boxes.content_box.origin;
    let mut right = first_child.boxes.margin_box.origin.x + first_child.boxes.margin_box.size.width;
    let mut bottom =
        first_child.boxes.margin_box.origin.y + first_child.boxes.margin_box.size.height;
    for child in fragment.children.iter().skip(1) {
        right = right.max(child.boxes.margin_box.origin.x + child.boxes.margin_box.size.width);
        bottom = bottom.max(child.boxes.margin_box.origin.y + child.boxes.margin_box.size.height);
    }
    fragment.boxes = inline_fragment_box_model(
        fragment.boxes.margin_box.origin,
        fragment.style,
        (right - content_origin.x).max(0.0),
        (bottom - content_origin.y).max(0.0),
    );
}

fn inline_fragment_box_model(
    origin: Point,
    style: ComputedStyle,
    content_width: f32,
    content_height: f32,
) -> BoxModel {
    let border_x = origin.x + style.margin.left;
    let border_y = origin.y + style.margin.top;
    let padding_x = border_x + style.border_width.left;
    let padding_y = border_y + style.border_width.top;
    let content_x = padding_x + style.padding.left;
    let content_y = padding_y + style.padding.top;
    let content_box = Rect::new(content_x, content_y, content_width, content_height);
    let padding_box = Rect::new(
        padding_x,
        padding_y,
        content_width + style.padding.horizontal(),
        content_height + style.padding.vertical(),
    );
    let border_box = Rect::new(
        border_x,
        border_y,
        padding_box.size.width + style.border_width.horizontal(),
        padding_box.size.height + style.border_width.vertical(),
    );
    let margin_box = Rect::new(
        origin.x,
        origin.y,
        border_box.size.width + style.margin.horizontal(),
        border_box.size.height + style.margin.vertical(),
    );
    BoxModel {
        margin_box,
        border_box,
        padding_box,
        content_box,
    }
}

fn translate_fragment_y(fragment: &mut Fragment, delta: f32) {
    fragment.boxes.margin_box.origin.y += delta;
    fragment.boxes.border_box.origin.y += delta;
    fragment.boxes.padding_box.origin.y += delta;
    fragment.boxes.content_box.origin.y += delta;
    if let Some(line_box) = &mut fragment.line_box {
        line_box.rect.origin.y += delta;
    }
    for child in &mut fragment.children {
        translate_fragment_y(child, delta);
    }
}

fn snapshot_layout_node(node: &LayoutNode, depth: usize, output: &mut String) {
    let dom = node
        .dom_node
        .map(|node| node.to_string())
        .unwrap_or_else(|| "-".into());
    let kind = match &node.kind {
        LayoutNodeKind::Root => "root".to_string(),
        LayoutNodeKind::Box => "box".to_string(),
        LayoutNodeKind::Text(run) => format!("text:{}", run.text),
    };
    output.push_str(&format!(
        "{}layout={}|dom={dom}|kind={kind}|children={}\n",
        " ".repeat(depth),
        node.id.index(),
        node.children.len()
    ));
    for child in &node.children {
        snapshot_layout_node(child, depth + 1, output);
    }
}

fn snapshot_style_node(node: &LayoutNode, output: &mut String) {
    if let Some(dom) = node.dom_node {
        let style = node.style;
        output.push_str(&format!(
            "dom={dom}|w={:?}|h={:?}|m={:.1},{:.1},{:.1},{:.1}|b={:.1},{:.1},{:.1},{:.1}|p={:.1},{:.1},{:.1},{:.1}|bg={:02x}{:02x}{:02x}{:02x}|bc={:02x}{:02x}{:02x}{:02x}|none={}\n",
            style.width,
            style.height,
            style.margin.top,
            style.margin.right,
            style.margin.bottom,
            style.margin.left,
            style.border_width.top,
            style.border_width.right,
            style.border_width.bottom,
            style.border_width.left,
            style.padding.top,
            style.padding.right,
            style.padding.bottom,
            style.padding.left,
            style.background.r,
            style.background.g,
            style.background.b,
            style.background.a,
            style.border_color.r,
            style.border_color.g,
            style.border_color.b,
            style.border_color.a,
            style.display_none,
        ));
    }
    for child in &node.children {
        snapshot_style_node(child, output);
    }
}

fn snapshot_fragment(fragment: &Fragment, depth: usize, output: &mut String) {
    let dom = fragment
        .dom_node
        .map(|node| node.to_string())
        .unwrap_or_else(|| "-".into());
    let text_range = fragment
        .text_range
        .map(|range| format!("{}..{}", range.start, range.end))
        .unwrap_or_else(|| "-".into());
    let line = fragment
        .line_box
        .map(|line| format!("{}:{}", line.ordinal, rect_snapshot(line.rect)))
        .unwrap_or_else(|| "-".into());
    output.push_str(&format!(
        "{}fragment={}|ordinal={}|layout={}|dom={dom}|kind={:?}|range={text_range}|line={line}|margin={}|border={}|padding={}|content={}\n",
        " ".repeat(depth),
        fragment.id.index(),
        fragment.ordinal.index(),
        fragment.layout_node.index(),
        fragment.kind,
        rect_snapshot(fragment.boxes.margin_box),
        rect_snapshot(fragment.boxes.border_box),
        rect_snapshot(fragment.boxes.padding_box),
        rect_snapshot(fragment.boxes.content_box),
    ));
    for child in &fragment.children {
        snapshot_fragment(child, depth + 1, output);
    }
}

fn rect_snapshot(rect: Rect) -> String {
    format!(
        "{:.1},{:.1},{:.1},{:.1}",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_dom::{ElementData, NodeKind};
    use std::collections::BTreeMap;

    fn element(name: &str, style: Option<&str>) -> NodeKind {
        let mut attributes = BTreeMap::new();
        if let Some(style) = style {
            attributes.insert("style".into(), style.into());
        }

        NodeKind::Element(ElementData::html(name).with_attributes(attributes))
    }

    #[test]
    fn text_run_reuses_segmentation_without_changing_cluster_or_intrinsic_results() {
        let run = TextRun::new("a e\u{301} bc 😀".into());
        assert_eq!(run.shaped.clusters.len(), 8);
        assert_eq!(
            run.font_runs,
            font_runs(&run.text, &FontFallbackChain::default())
        );
        assert_eq!(run.intrinsic_sizes().min_content, 16.0);
        assert_eq!(run.intrinsic_sizes().max_content, run.advance);
    }

    #[test]
    fn layout_identity_is_distinct_and_fragments_point_back_to_it() {
        let mut doc = Document::new();
        let element = doc.append_new(doc.root(), element("div", None)).unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let layout_node = &output.tree.root.children[0];
        let fragment = &output.fragments.root.children[0];

        assert_eq!(layout_node.dom_node, Some(element));
        assert_eq!(fragment.dom_node, Some(element));
        assert_eq!(fragment.layout_node, layout_node.id);
    }

    #[test]
    fn author_stylesheet_participates_in_layout() {
        let mut doc = Document::new();
        let style = doc.append_new(doc.root(), element("style", None)).unwrap();
        doc.append_new(style, NodeKind::Text(".card { width:42px; }".into()))
            .unwrap();
        let mut attributes = BTreeMap::new();
        attributes.insert("class".into(), "card".into());
        doc.append_new(
            doc.root(),
            NodeKind::Element(ElementData::html("div").with_attributes(attributes)),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        assert_eq!(
            output.fragments.root.children[0]
                .boxes
                .content_box
                .size
                .width,
            42.0
        );
    }

    #[test]
    fn box_model_tracks_margin_border_padding_and_content_boxes() {
        let mut doc = Document::new();
        doc.append_new(
            doc.root(),
            element(
                "div",
                Some(
                    "width:100px;height:20px;margin:5px;padding:10px;\
                     border-width:2px;border-color:#000000",
                ),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let fragment = &output.fragments.root.children[0];

        assert_eq!(fragment.boxes.margin_box, Rect::new(0.0, 0.0, 134.0, 54.0));
        assert_eq!(fragment.boxes.border_box, Rect::new(5.0, 5.0, 124.0, 44.0));
        assert_eq!(fragment.boxes.padding_box, Rect::new(7.0, 7.0, 120.0, 40.0));
        assert_eq!(
            fragment.boxes.content_box,
            Rect::new(17.0, 17.0, 100.0, 20.0)
        );
    }

    #[test]
    fn display_flex_places_fixed_items_in_one_source_order_row() {
        let mut doc = Document::new();
        let container = doc
            .append_new(doc.root(), element("div", Some("display:flex;width:120px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:30px;height:15px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert!(output.tree.root.children[0].style.display_flex);
        assert_eq!(container.children.len(), 2);
        assert_eq!(
            container.children[0].boxes.border_box,
            Rect::new(0.0, 0.0, 20.0, 10.0)
        );
        assert_eq!(
            container.children[1].boxes.border_box,
            Rect::new(20.0, 0.0, 30.0, 15.0)
        );
        assert_eq!(container.boxes.content_box.size.height, 15.0);
    }

    #[test]
    fn fixed_css_grid_places_explicit_items_and_uses_track_gaps() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:grid;width:110px;grid-template-columns:40px 60px;grid-template-rows:20px 30px;column-gap:10px;row-gap:5px",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some("grid-row-start:1;grid-column-start:1;background:#112233"),
            ),
        )
        .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some("grid-row-start:2;grid-column-start:2;background:#445566"),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert!(output.tree.root.children[0].style.display_grid);
        assert_eq!(container.boxes.content_box.size.height, 55.0);
        assert_eq!(
            container.children[0].boxes.border_box,
            Rect::new(0.0, 0.0, 40.0, 20.0)
        );
        assert_eq!(
            container.children[1].boxes.border_box,
            Rect::new(50.0, 25.0, 60.0, 30.0)
        );
    }

    #[test]
    fn fixed_css_grid_supports_spans_and_preserves_source_order() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:grid;width:100px;grid-template-columns:20px 30px 40px;grid-template-rows:10px 15px;column-gap:4px;row-gap:6px",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some(
                    "grid-row-start:1;grid-column-start:1;grid-row-end:span 2;grid-column-end:span 2",
                ),
            ),
        )
        .unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-row-start:1;grid-column-start:1")),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children.len(), 2);
        assert_eq!(
            container.children[0].boxes.border_box,
            Rect::new(0.0, 0.0, 54.0, 31.0)
        );
        assert_eq!(
            container.children[1].boxes.border_box,
            Rect::new(0.0, 0.0, 20.0, 10.0)
        );
    }

    #[test]
    fn fixed_css_grid_stretch_respects_physical_item_margins() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:grid;width:60px;grid-template-columns:60px;grid-template-rows:30px",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some(
                    "grid-row-start:1;grid-column-start:1;margin:2px 3px 4px 5px;padding:1px;border-width:1px",
                ),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let item = &output.fragments.root.children[0].children[0];

        assert_eq!(item.boxes.margin_box, Rect::new(0.0, 0.0, 60.0, 30.0));
        assert_eq!(item.boxes.border_box, Rect::new(5.0, 2.0, 52.0, 24.0));
        assert_eq!(
            item.boxes.content_box.size,
            Size {
                width: 48.0,
                height: 20.0
            }
        );
    }

    #[test]
    fn css_grid_auto_places_items_in_row_major_order() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:grid;grid-template-columns:40px 60px;grid-template-rows:20px 30px",
                    ),
                ),
            )
            .unwrap();
        for _ in 0..3 {
            doc.append_new(container, element("div", None)).unwrap();
        }

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box, Rect::new(0.0, 0.0, 40.0, 20.0));
        assert_eq!(container.children[1].boxes.border_box, Rect::new(40.0, 0.0, 60.0, 20.0));
        assert_eq!(container.children[2].boxes.border_box, Rect::new(0.0, 20.0, 40.0, 30.0));
    }

    #[test]
    fn css_grid_explicit_items_reserve_cells_before_auto_items() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:grid;grid-template-columns:40px 60px;grid-template-rows:20px 30px",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", None)).unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-row-start:1;grid-column-start:1")),
        )
        .unwrap();
        doc.append_new(container, element("div", None)).unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box, Rect::new(40.0, 0.0, 60.0, 20.0));
        assert_eq!(container.children[1].boxes.border_box, Rect::new(0.0, 0.0, 40.0, 20.0));
        assert_eq!(container.children[2].boxes.border_box, Rect::new(0.0, 20.0, 40.0, 30.0));
    }

    #[test]
    fn css_grid_auto_placement_honors_partial_axis_and_spans() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:grid;grid-template-columns:20px 30px 40px;grid-template-rows:10px 15px",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-row-start:1;grid-column-start:1")),
        )
        .unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-column-end:span 2")),
        )
        .unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-row-start:2")),
        )
        .unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-column-start:3")),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[1].boxes.border_box, Rect::new(20.0, 0.0, 70.0, 10.0));
        assert_eq!(container.children[2].boxes.border_box, Rect::new(0.0, 10.0, 20.0, 15.0));
        assert_eq!(container.children[3].boxes.border_box, Rect::new(50.0, 10.0, 40.0, 15.0));
    }

    #[test]
    fn css_grid_auto_placement_fails_closed_when_explicit_grid_is_full() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:grid;grid-template-columns:40px;grid-template-rows:20px"),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element("div", Some("grid-row-start:1;grid-column-start:1")),
        )
        .unwrap();
        doc.append_new(container, element("div", None)).unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert!(container.children.is_empty());
        assert_eq!(container.boxes.content_box.size.height, 20.0);
    }


    #[test]
    fn flex_direction_row_reverse_places_source_order_items_from_the_right() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-direction:row-reverse;width:100px"),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:30px;height:10px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.x, 80.0);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 50.0);
    }

    #[test]
    fn flex_direction_row_reverse_preserves_physical_horizontal_margins() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-direction:row-reverse;width:60px"),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some("width:10px;height:10px;margin-left:2px;margin-right:3px"),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let item = &output.fragments.root.children[0].children[0];

        assert_eq!(item.boxes.border_box.origin.x, 47.0);
        assert_eq!(item.boxes.margin_box.origin.x, 45.0);
        assert_eq!(item.boxes.margin_box.size.width, 15.0);
    }

    #[test]
    fn flex_direction_row_reverse_applies_justify_content_logically() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-direction:row-reverse;width:60px;justify-content:flex-end",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:10px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:10px;height:10px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.x, 10.0);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 0.0);
    }

    #[test]
    fn flex_direction_row_reverse_reuses_source_order_line_collection() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-direction:row-reverse;flex-wrap:wrap;width:100px;column-gap:10px",
                    ),
                ),
            )
            .unwrap();
        for _ in 0..3 {
            doc.append_new(container, element("div", Some("width:40px;height:10px")))
                .unwrap();
        }

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.x, 60.0);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 10.0);
        assert_eq!(container.children[2].boxes.border_box.origin.x, 60.0);
        assert_eq!(container.children[2].boxes.border_box.origin.y, 10.0);
    }

    #[test]
    fn flex_wrap_creates_multiple_lines_and_uses_row_gap() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-wrap:wrap;width:100px;column-gap:10px;row-gap:5px"),
                ),
            )
            .unwrap();
        for height in [10, 20, 15] {
            doc.append_new(
                container,
                element("div", Some(&format!("width:40px;height:{height}px"))),
            )
            .unwrap();
        }

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(
            container.children[0].boxes.border_box.origin,
            Point { x: 0.0, y: 0.0 }
        );
        assert_eq!(
            container.children[1].boxes.border_box.origin,
            Point { x: 50.0, y: 0.0 }
        );
        assert_eq!(
            container.children[2].boxes.border_box.origin,
            Point { x: 0.0, y: 25.0 }
        );
        assert_eq!(container.boxes.content_box.size.height, 40.0);
    }

    #[test]
    fn row_reverse_and_wrap_reverse_compose_without_reordering_fragments() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-direction:row-reverse;flex-wrap:wrap-reverse;width:100px;height:60px;align-content:flex-start",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.x, 40.0);
        assert_eq!(container.children[0].boxes.border_box.origin.y, 50.0);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 40.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 30.0);
    }

    #[test]
    fn flex_wrap_reverse_stacks_lines_from_the_container_cross_end() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-wrap:wrap-reverse;width:100px;height:60px;align-content:flex-start",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.y, 50.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 30.0);
    }

    #[test]
    fn flex_wrap_reverse_flips_item_cross_start_without_swapping_physical_margins() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-wrap:wrap-reverse;width:100px;height:60px;align-items:flex-start",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some("width:60px;height:10px;margin-top:2px;margin-bottom:3px"),
            ),
        )
        .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let first = &output.fragments.root.children[0].children[0];

        assert_eq!(first.boxes.border_box.origin.y, 47.0);
        assert_eq!(first.boxes.margin_box.origin.y, 45.0);
        assert_eq!(first.boxes.margin_box.size.height, 15.0);
    }

    #[test]
    fn flex_wrap_reverse_supports_measured_auto_height_items() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-wrap:wrap-reverse;width:100px;height:60px;align-content:flex-start",
                    ),
                ),
            )
            .unwrap();
        let first = doc
            .append_new(container, element("div", Some("width:60px")))
            .unwrap();
        doc.append_new(first, element("div", Some("height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.size.height, 10.0);
        assert_eq!(container.children[0].boxes.border_box.origin.y, 50.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 30.0);
    }

    #[test]
    fn flex_wrap_distributes_grow_per_line() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-wrap:wrap;width:80px;column-gap:10px"),
                ),
            )
            .unwrap();
        for _ in 0..3 {
            doc.append_new(
                container,
                element("div", Some("width:30px;height:10px;flex-grow:1")),
            )
            .unwrap();
        }

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.size.width, 35.0);
        assert_eq!(container.children[1].boxes.border_box.size.width, 35.0);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 45.0);
        assert_eq!(container.children[2].boxes.border_box.size.width, 80.0);
        assert_eq!(container.children[2].boxes.border_box.origin.y, 10.0);
    }

    #[test]
    fn wrapped_definite_height_container_stretches_flex_lines_by_default() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-wrap:wrap;width:100px;height:60px"),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.boxes.content_box.size.height, 60.0);
        assert_eq!(container.children[0].boxes.border_box.origin.y, 0.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 25.0);
    }

    #[test]
    fn align_content_center_positions_wrapped_lines_in_definite_height() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-wrap:wrap;width:100px;height:60px;align-content:center",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.y, 15.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 25.0);
    }

    #[test]
    fn align_content_space_between_adds_to_row_gap() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some(
                        "display:flex;flex-wrap:wrap;width:100px;height:60px;row-gap:5px;align-content:space-between",
                    ),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.y, 0.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 40.0);
    }

    #[test]
    fn wrapped_auto_height_items_measure_content_and_stretch_per_line() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-wrap:wrap;width:100px;height:60px"),
                ),
            )
            .unwrap();
        let first = doc
            .append_new(container, element("div", Some("width:60px")))
            .unwrap();
        doc.append_new(first, element("div", Some("height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];
        let first = &container.children[0];

        assert_eq!(first.boxes.border_box.size.height, 25.0);
        assert_eq!(first.children[0].boxes.border_box.size.height, 10.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 25.0);
    }

    #[test]
    fn wrapped_auto_height_non_stretch_item_keeps_measured_height() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-wrap:wrap;width:100px;height:60px;align-items:center"),
                ),
            )
            .unwrap();
        let first = doc
            .append_new(container, element("div", Some("width:60px")))
            .unwrap();
        doc.append_new(first, element("div", Some("height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.size.height, 10.0);
        assert_eq!(container.children[0].boxes.border_box.origin.y, 7.5);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 32.5);
    }

    #[test]
    fn wrapped_auto_height_stretch_respects_item_max_height() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;flex-wrap:wrap;width:100px;height:60px"),
                ),
            )
            .unwrap();
        let first = doc
            .append_new(
                container,
                element("div", Some("width:60px;max-height:15px")),
            )
            .unwrap();
        doc.append_new(first, element("div", Some("height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:60px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let first = &output.fragments.root.children[0].children[0];

        assert_eq!(first.boxes.content_box.size.height, 15.0);
        assert_eq!(first.boxes.border_box.size.height, 15.0);
    }

    #[test]
    fn auto_height_wrapped_container_uses_measured_item_cross_sizes() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element("div", Some("display:flex;flex-wrap:wrap;width:100px")),
            )
            .unwrap();
        let first = doc
            .append_new(container, element("div", Some("width:60px")))
            .unwrap();
        doc.append_new(first, element("div", Some("height:10px")))
            .unwrap();
        let second = doc
            .append_new(container, element("div", Some("width:60px")))
            .unwrap();
        doc.append_new(second, element("div", Some("height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.boxes.content_box.size.height, 30.0);
        assert_eq!(container.children[0].boxes.border_box.size.height, 10.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 10.0);
        assert_eq!(container.children[1].boxes.border_box.size.height, 20.0);
    }

    #[test]
    fn definite_height_flex_container_stretches_auto_height_item() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element("div", Some("display:flex;width:100px;height:60px")),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];
        let item = &container.children[0];

        assert_eq!(item.boxes.border_box.origin.y, 0.0);
        assert_eq!(item.boxes.border_box.size.height, 60.0);
        assert_eq!(item.boxes.content_box.size.height, 60.0);
    }

    #[test]
    fn align_self_stretch_enables_auto_height_override() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;height:60px;align-items:center"),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element("div", Some("width:20px;align-self:stretch")),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let item = &output.fragments.root.children[0].children[0];

        assert_eq!(item.boxes.border_box.origin.y, 0.0);
        assert_eq!(item.boxes.border_box.size.height, 60.0);
    }

    #[test]
    fn stretch_auto_height_respects_edges_and_cross_size_limits() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element("div", Some("display:flex;width:100px;height:60px")),
            )
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some(
                    "width:20px;margin-top:5px;margin-bottom:5px;padding-top:3px;padding-bottom:3px;border-width:2px;max-height:30px",
                ),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let item = &output.fragments.root.children[0].children[0];

        assert_eq!(item.boxes.border_box.origin.y, 5.0);
        assert_eq!(item.boxes.content_box.size.height, 30.0);
        assert_eq!(item.boxes.border_box.size.height, 40.0);
        assert_eq!(item.boxes.margin_box.size.height, 50.0);
    }

    #[test]
    fn auto_height_item_without_effective_stretch_remains_fail_closed() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;height:60px;align-items:flex-start"),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );

        assert!(output.fragments.root.children[0].children.is_empty());
    }

    #[test]
    fn stretched_auto_height_nested_flex_item_has_a_definite_inner_cross_size() {
        let mut doc = Document::new();
        let outer = doc
            .append_new(
                doc.root(),
                element("div", Some("display:flex;width:100px;height:60px")),
            )
            .unwrap();
        let inner = doc
            .append_new(outer, element("div", Some("display:flex;width:40px")))
            .unwrap();
        doc.append_new(inner, element("div", Some("width:10px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let inner = &output.fragments.root.children[0].children[0];
        let nested = &inner.children[0];

        assert_eq!(inner.boxes.content_box.size.height, 60.0);
        assert_eq!(nested.boxes.border_box.size.height, 60.0);
    }

    #[test]
    fn align_self_overrides_container_cross_axis_alignment_per_item() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;height:60px;align-items:center"),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element("div", Some("width:20px;height:10px;align-self:flex-end")),
        )
        .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.y, 50.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 20.0);
    }

    #[test]
    fn align_self_auto_uses_container_align_items() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;height:60px;align-items:flex-end"),
                ),
            )
            .unwrap();
        doc.append_new(
            container,
            element("div", Some("width:20px;height:10px;align-self:auto")),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let item = &output.fragments.root.children[0].children[0];

        assert_eq!(item.boxes.border_box.origin.y, 50.0);
    }

    #[test]
    fn align_items_positions_explicit_height_items_on_the_cross_axis() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;height:60px;align-items:center"),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.y, 25.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 20.0);
        assert_eq!(container.boxes.content_box.size.height, 60.0);
    }

    #[test]
    fn align_items_uses_auto_container_min_height_as_the_line_cross_size() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;min-height:50px;align-items:flex-end"),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:10px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.origin.y, 40.0);
        assert_eq!(container.children[1].boxes.border_box.origin.y, 30.0);
        assert_eq!(container.boxes.content_box.size.height, 50.0);
    }

    #[test]
    fn align_items_stretch_preserves_explicit_item_heights_in_this_slice() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;height:60px;align-items:stretch"),
                ),
            )
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:10px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let item = &output.fragments.root.children[0].children[0];

        assert_eq!(item.boxes.border_box.origin.y, 0.0);
        assert_eq!(item.boxes.border_box.size.height, 10.0);
    }

    #[test]
    fn column_gap_and_gap_shorthand_space_single_row_flex_items() {
        let mut column_doc = Document::new();
        let column = column_doc
            .append_new(
                column_doc.root(),
                element("div", Some("display:flex;width:100px;column-gap:10px")),
            )
            .unwrap();
        column_doc
            .append_new(column, element("div", Some("width:20px;height:10px")))
            .unwrap();
        column_doc
            .append_new(column, element("div", Some("width:20px;height:10px")))
            .unwrap();

        let column_output = layout_document(
            &column_doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let column = &column_output.fragments.root.children[0];
        assert_eq!(column.children[0].boxes.border_box.origin.x, 0.0);
        assert_eq!(column.children[1].boxes.border_box.origin.x, 30.0);

        let mut shorthand_doc = Document::new();
        let shorthand = shorthand_doc
            .append_new(
                shorthand_doc.root(),
                element("div", Some("display:flex;width:100px;gap:7px 12px")),
            )
            .unwrap();
        shorthand_doc
            .append_new(shorthand, element("div", Some("width:20px;height:10px")))
            .unwrap();
        shorthand_doc
            .append_new(shorthand, element("div", Some("width:20px;height:10px")))
            .unwrap();

        let shorthand_output = layout_document(
            &shorthand_doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let shorthand = &shorthand_output.fragments.root.children[0];
        assert_eq!(shorthand.children[0].boxes.border_box.origin.x, 0.0);
        assert_eq!(shorthand.children[1].boxes.border_box.origin.x, 32.0);
        assert_eq!(shorthand.style.row_gap, 7.0);
        assert_eq!(shorthand.style.column_gap, 12.0);
    }

    #[test]
    fn column_gap_is_reserved_before_flex_grow() {
        let mut doc = Document::new();
        let container = doc
            .append_new(
                doc.root(),
                element("div", Some("display:flex;width:100px;column-gap:10px")),
            )
            .unwrap();
        for _ in 0..2 {
            doc.append_new(
                container,
                element("div", Some("width:20px;height:10px;flex-grow:1")),
            )
            .unwrap();
        }

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children[0].boxes.border_box.size.width, 45.0);
        assert_eq!(container.children[1].boxes.border_box.size.width, 45.0);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 55.0);
    }

    #[test]
    fn justify_content_positions_fixed_flex_items_on_the_main_axis() {
        let mut centered_doc = Document::new();
        let centered = centered_doc
            .append_new(
                centered_doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;justify-content:center"),
                ),
            )
            .unwrap();
        centered_doc
            .append_new(centered, element("div", Some("width:20px;height:10px")))
            .unwrap();
        centered_doc
            .append_new(centered, element("div", Some("width:20px;height:10px")))
            .unwrap();

        let centered_output = layout_document(
            &centered_doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let centered = &centered_output.fragments.root.children[0];
        assert_eq!(centered.children[0].boxes.border_box.origin.x, 30.0);
        assert_eq!(centered.children[1].boxes.border_box.origin.x, 50.0);

        let mut between_doc = Document::new();
        let between = between_doc
            .append_new(
                between_doc.root(),
                element(
                    "div",
                    Some("display:flex;width:100px;justify-content:space-between"),
                ),
            )
            .unwrap();
        between_doc
            .append_new(between, element("div", Some("width:20px;height:10px")))
            .unwrap();
        between_doc
            .append_new(between, element("div", Some("width:20px;height:10px")))
            .unwrap();

        let between_output = layout_document(
            &between_doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let between = &between_output.fragments.root.children[0];
        assert_eq!(between.children[0].boxes.border_box.origin.x, 0.0);
        assert_eq!(between.children[1].boxes.border_box.origin.x, 80.0);
    }

    #[test]
    fn flex_grow_and_shrink_change_used_item_widths() {
        let mut grow_doc = Document::new();
        let grow_container = grow_doc
            .append_new(
                grow_doc.root(),
                element("div", Some("display:flex;width:100px")),
            )
            .unwrap();
        grow_doc
            .append_new(
                grow_container,
                element(
                    "div",
                    Some("width:20px;height:10px;flex-grow:1;flex-shrink:1"),
                ),
            )
            .unwrap();
        grow_doc
            .append_new(
                grow_container,
                element(
                    "div",
                    Some("width:20px;height:10px;flex-grow:3;flex-shrink:1"),
                ),
            )
            .unwrap();

        let grow = layout_document(
            &grow_doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let grow_container = &grow.fragments.root.children[0];
        assert_eq!(grow_container.children[0].boxes.border_box.size.width, 35.0);
        assert_eq!(grow_container.children[1].boxes.border_box.size.width, 65.0);
        assert_eq!(grow_container.children[1].boxes.border_box.origin.x, 35.0);

        let mut shrink_doc = Document::new();
        let shrink_container = shrink_doc
            .append_new(
                shrink_doc.root(),
                element("div", Some("display:flex;width:45px")),
            )
            .unwrap();
        shrink_doc
            .append_new(
                shrink_container,
                element("div", Some("width:40px;height:10px;flex-shrink:1")),
            )
            .unwrap();
        shrink_doc
            .append_new(
                shrink_container,
                element("div", Some("width:20px;height:10px;flex-shrink:1")),
            )
            .unwrap();

        let shrink = layout_document(
            &shrink_doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let shrink_container = &shrink.fragments.root.children[0];
        assert_eq!(
            shrink_container.children[0].boxes.border_box.size.width,
            30.0
        );
        assert_eq!(
            shrink_container.children[1].boxes.border_box.size.width,
            15.0
        );
        assert_eq!(shrink_container.children[1].boxes.border_box.origin.x, 30.0);
    }

    #[test]
    fn flex_sizing_fails_closed_when_post_flex_limits_need_redistribution() {
        let mut doc = Document::new();
        let container = doc
            .append_new(doc.root(), element("div", Some("display:flex;width:100px")))
            .unwrap();
        doc.append_new(
            container,
            element(
                "div",
                Some("width:20px;max-width:30px;height:10px;flex-grow:1"),
            ),
        )
        .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert!(container.children.is_empty());
        assert_eq!(container.boxes.content_box.size.height, 0.0);
    }

    #[test]
    fn flex_ignores_whitespace_between_fixed_items() {
        let mut doc = Document::new();
        let container = doc
            .append_new(doc.root(), element("div", Some("display:flex;width:120px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px;height:10px")))
            .unwrap();
        doc.append_new(container, NodeKind::Text(" \n ".into()))
            .unwrap();
        doc.append_new(container, element("div", Some("width:30px;height:15px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert_eq!(container.children.len(), 2);
        assert_eq!(container.children[1].boxes.border_box.origin.x, 20.0);
    }

    #[test]
    fn unsupported_auto_sized_flex_items_do_not_fall_back_to_block_flow() {
        let mut doc = Document::new();
        let container = doc
            .append_new(doc.root(), element("div", Some("display:flex;width:120px")))
            .unwrap();
        doc.append_new(container, element("div", Some("width:20px")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let container = &output.fragments.root.children[0];

        assert!(container.children.is_empty());
        assert_eq!(container.boxes.content_box.size.height, 0.0);
    }

    #[test]
    fn display_none_nodes_do_not_enter_the_layout_or_fragment_trees() {
        let mut doc = Document::new();
        doc.append_new(doc.root(), element("div", Some("display:none")))
            .unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );

        assert!(output.tree.root.children.is_empty());
        assert!(output.fragments.root.children.is_empty());
    }

    #[test]
    fn text_runs_expose_intrinsic_sizes() {
        let run = TextRun::new("small verylongword".into());
        assert_eq!(
            run.intrinsic_sizes(),
            IntrinsicSizes {
                min_content: 96.0,
                max_content: 144.0,
            }
        );
    }

    #[test]
    fn nested_boxes_use_parent_content_as_containing_block() {
        let mut doc = Document::new();
        let parent = doc
            .append_new(doc.root(), element("div", Some("width:100px;padding:10px")))
            .unwrap();
        doc.append_new(parent, element("div", None)).unwrap();

        let output = layout_document(
            &doc,
            Size {
                width: 320.0,
                height: 200.0,
            },
        );
        let parent_fragment = &output.fragments.root.children[0];
        let child = &parent_fragment.children[0];
        assert_eq!(child.boxes.content_box.origin.x, 10.0);
        assert_eq!(child.boxes.content_box.size.width, 100.0);
    }

    #[test]
    fn snapshots_are_deterministic() {
        let mut doc = Document::new();
        doc.append_new(doc.root(), element("div", Some("width:20px")))
            .unwrap();
        let output = layout_document(
            &doc,
            Size {
                width: 100.0,
                height: 100.0,
            },
        );

        assert_eq!(output.tree.snapshot(), output.tree.snapshot());
        assert_eq!(output.tree.style_snapshot(), output.tree.style_snapshot());
        assert_eq!(output.fragments.snapshot(), output.fragments.snapshot());
    }

    #[test]
    fn narrow_text_produces_multiple_fragments_for_one_layout_node() {
        let mut doc = Document::new();
        let text_node = doc
            .append_new(doc.root(), NodeKind::Text("abcdefghij".into()))
            .unwrap();
        let output = layout_document(
            &doc,
            Size {
                width: 24.0,
                height: 200.0,
            },
        );
        let layout_node = &output.tree.root.children[0];
        let fragments = fragments_for_dom(&output.fragments, text_node);
        assert_eq!(fragments.len(), 4);
        assert!(
            fragments
                .iter()
                .all(|fragment| fragment.layout_node == layout_node.id)
        );
        assert_eq!(
            fragments
                .iter()
                .map(|fragment| fragment.ordinal.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(fragments[0].boxes.content_box.size.width, 24.0);
        assert_eq!(fragments[3].boxes.content_box.size.width, 8.0);
        assert_eq!(fragments[0].text_range, Some(TextRange::new(0, 3)));
        assert_eq!(fragments[1].text_range, Some(TextRange::new(3, 6)));
        assert_eq!(fragments[2].text_range, Some(TextRange::new(6, 9)));
        assert_eq!(fragments[3].text_range, Some(TextRange::new(9, 10)));
        assert_eq!(fragments[0].line_box.unwrap().ordinal, 0);
        assert_eq!(fragments[3].line_box.unwrap().ordinal, 3);
    }

    #[test]
    fn fixed_advance_line_breaker_returns_stable_text_ranges() {
        let breaker = UnicodeLineBreaker;
        let run = TextRun::new("abcdefg".into());
        assert_eq!(
            breaker.break_text(&run, 24.0),
            vec![
                TextRange::new(0, 3),
                TextRange::new(3, 6),
                TextRange::new(6, 7),
            ]
        );
    }

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
        run.advance = run
            .shaped
            .clusters
            .iter()
            .map(|cluster| cluster.advance)
            .sum();
        let breaker = UnicodeLineBreaker;
        assert_eq!(
            breaker.break_text(&run, 16.0),
            vec![
                TextRange::new(0, 1),
                TextRange::new(1, 2),
                TextRange::new(2, 3)
            ]
        );
    }

    #[test]
    fn unicode_break_opportunities_cover_whitespace_hyphen_cjk_and_mandatory_breaks() {
        let opportunities = unicode_break_opportunities("a b-c中日\nq");
        for expected in [
            BreakOpportunity {
                index: 2,
                kind: BreakKind::Soft,
            },
            BreakOpportunity {
                index: 4,
                kind: BreakKind::Soft,
            },
            BreakOpportunity {
                index: 6,
                kind: BreakKind::Soft,
            },
            BreakOpportunity {
                index: 8,
                kind: BreakKind::Mandatory,
            },
        ] {
            assert!(opportunities.contains(&expected));
        }
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
    fn long_text_analysis_preserves_linear_sized_results() {
        let text = "ab cd-世界🇱🇹".repeat(2_000);
        let boundaries = grapheme_boundaries(&text);
        let opportunities = unicode_break_opportunities(&text);
        let runs = font_runs(&text, &FontFallbackChain::default());

        assert!(!boundaries.is_empty());
        assert!(boundaries.len() <= text.chars().count() + 1);
        assert!(opportunities.len() <= text.chars().count());
        assert!(!runs.is_empty());
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
            vec![BreakOpportunity {
                index: 3,
                kind: BreakKind::Mandatory
            }]
        );
    }

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
        assert_eq!(
            runs[0],
            BidiRun {
                range: TextRange::new(0, 4),
                level: BidiLevel::new(0)
            }
        );
        assert_eq!(
            runs[1],
            BidiRun {
                range: TextRange::new(4, 8),
                level: BidiLevel::new(1)
            }
        );
        assert_eq!(
            runs[2],
            BidiRun {
                range: TextRange::new(8, 12),
                level: BidiLevel::new(0)
            }
        );
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
        assert!(
            runs.iter()
                .all(|run| run.range.end <= "a e\u{301} שלום".chars().count())
        );
        assert_eq!(grapheme_boundaries("e\u{301}"), vec![0, 2]);
    }

    #[test]
    fn font_fallback_keeps_latin_and_cyrillic_in_primary_face() {
        let chain = FontFallbackChain::default();
        let runs = font_runs("Hello Привет", &chain);
        assert_eq!(
            runs,
            vec![FontRun {
                range: TextRange::new(0, 12),
                face: FontFaceId::new(0)
            }]
        );
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
        assert_eq!(
            runs[1],
            FontRun {
                range: TextRange::new(1, 5),
                face: FontFaceId::new(3)
            }
        );
    }

    #[test]
    fn font_fallback_has_deterministic_last_resort() {
        let chain = FontFallbackChain::default();
        let runs = font_runs("\u{10300}", &chain);
        assert_eq!(
            runs,
            vec![FontRun {
                range: TextRange::new(0, 1),
                face: FontFaceId::new(4)
            }]
        );
        assert_eq!(
            chain.face(FontFaceId::new(4)).unwrap().family.0,
            "Rarog LastResort"
        );
    }

    #[test]
    fn text_run_exposes_font_runs_without_changing_source_ranges() {
        let run = TextRun::new("abc שלום".into());
        assert_eq!(run.font_runs.len(), 2);
        assert_eq!(run.font_runs[0].range, TextRange::new(0, 4));
        assert_eq!(run.font_runs[1].range, TextRange::new(4, 8));
        assert_eq!(run.character_count(), 8);
    }

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
                    range: TextRange::new(4, 8),
                    face: FontFaceId::new(1),
                    level: BidiLevel::new(1),
                },
                ShapingRun {
                    range: TextRange::new(8, 9),
                    face: FontFaceId::new(1),
                    level: BidiLevel::new(0),
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
            is_grapheme_boundary(text, run.range.start) && is_grapheme_boundary(text, run.range.end)
        }));
        assert_eq!(runs.first().unwrap().range.start, 0);
        assert_eq!(runs.last().unwrap().range.end, text.chars().count());
        assert!(
            runs.windows(2)
                .all(|pair| pair[0].range.end == pair[1].range.start)
        );
    }

    #[test]
    fn text_run_exposes_stable_shaping_segments() {
        let run = TextRun::new("abc שלום 世界".into());
        let segments = run.shaping_runs();
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].direction(), TextDirection::Ltr);
        assert_eq!(segments[1].direction(), TextDirection::Rtl);
        assert_eq!(segments[2].direction(), TextDirection::Ltr);
        assert_eq!(segments[3].direction(), TextDirection::Ltr);
    }

    #[test]
    fn shaping_backend_returns_glyph_ids_advances_offsets_and_source_mapping() {
        let text = "a👩🏽\u{200d}💻b";
        let fallback = FontFallbackChain::default();
        let runs = shaping_runs(text, &fallback);
        let backend = FixedTextShaper::default();
        let shaped = runs
            .iter()
            .map(|run| {
                backend.shape_run(
                    text,
                    &ShapingRequest::bootstrap(text, *run),
                    fallback.face(run.face).unwrap(),
                )
            })
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
        let shaped = FixedTextShaper::default().shape_run(
            "ab",
            &ShapingRequest::bootstrap("ab", run),
            &face,
        );
        assert_eq!(shaped.metrics, metrics);
        assert_eq!(shaped.advance, 22.0);
        assert_eq!(
            shaped
                .glyphs
                .iter()
                .map(|glyph| glyph.advance)
                .collect::<Vec<_>>(),
            vec![11.0, 11.0]
        );
    }

    #[test]
    fn rtl_shaping_run_returns_visual_glyph_order_with_logical_source_mapping() {
        let text = "אב";
        let fallback = FontFallbackChain::default();
        let run = shaping_runs(text, &fallback)[0];
        assert_eq!(run.direction(), TextDirection::Rtl);
        let shaped = FixedTextShaper::default().shape_run(
            text,
            &ShapingRequest::bootstrap(text, run),
            fallback.face(run.face).unwrap(),
        );
        assert_eq!(shaped.glyphs.len(), 2);
        assert_eq!(shaped.glyphs[0].source, TextRange::new(1, 2));
        assert_eq!(shaped.glyphs[1].source, TextRange::new(0, 1));
    }

    #[test]
    fn text_run_can_shape_all_segments_through_backend_boundary() {
        let fallback = FontFallbackChain::default();
        let run = TextRun::with_fallback("abc שלום 世界".into(), &fallback);
        let shaped = run.shape_with_backend(&fallback, &FixedTextShaper::default());
        assert_eq!(shaped.len(), 4);
        assert_eq!(shaped[0].run.face, FontFaceId::new(0));
        assert_eq!(shaped[1].run.face, FontFaceId::new(1));
        assert_eq!(shaped[2].run.face, FontFaceId::new(1));
        assert_eq!(shaped[3].run.face, FontFaceId::new(2));
        assert_eq!(shaped[1].run.direction(), TextDirection::Rtl);
        assert_eq!(shaped[2].run.direction(), TextDirection::Ltr);
        assert!(shaped.iter().all(|segment| !segment.glyphs.is_empty()));
    }

    #[test]
    fn shaping_request_infers_script_without_changing_source_range() {
        let text = "abc Привет שלום مرحبا 世界 👩🏽\u{200d}💻";
        let fallback = FontFallbackChain::default();
        let run = TextRun::with_fallback(text.into(), &fallback);
        let requests = run.shaping_requests();
        assert!(
            requests
                .iter()
                .any(|request| request.script == ShapingScript::Latin)
        );
        assert!(
            requests
                .iter()
                .any(|request| request.script == ShapingScript::Cyrillic)
        );
        assert!(
            requests
                .iter()
                .any(|request| request.script == ShapingScript::Hebrew)
        );
        assert!(
            requests
                .iter()
                .any(|request| request.script == ShapingScript::Arabic)
        );
        assert!(
            requests
                .iter()
                .any(|request| request.script == ShapingScript::Han)
        );
        assert!(
            requests
                .iter()
                .any(|request| request.script == ShapingScript::Emoji)
        );
        assert!(
            requests
                .iter()
                .all(|request| request.run.range.start < request.run.range.end)
        );
    }

    #[test]
    fn language_tag_has_deterministic_und_default_and_normalization() {
        assert_eq!(LanguageTag::default().as_str(), "und");
        assert_eq!(LanguageTag::new("").as_str(), "und");
        assert_eq!(LanguageTag::new("RU-ru").as_str(), "ru-ru");
    }

    #[test]
    fn shaping_request_carries_features_and_variation_coordinates() {
        let run = ShapingRun {
            range: TextRange::new(0, 3),
            face: FontFaceId::new(0),
            level: BidiLevel::new(0),
        };
        let mut request = ShapingRequest::bootstrap("abc", run);
        request.language = LanguageTag::new("en");
        request.features.push(OpenTypeFeature {
            tag: OpenTypeTag::from_bytes(*b"liga"),
            value: 1,
        });
        request.variations.push(VariationCoordinate {
            axis: OpenTypeTag::from_bytes(*b"wght"),
            value: 650.0,
        });
        assert_eq!(request.script, ShapingScript::Latin);
        assert_eq!(request.language.as_str(), "en");
        assert_eq!(
            request.features[0].tag.value(),
            u32::from_be_bytes(*b"liga")
        );
        assert_eq!(
            request.variations[0].axis.value(),
            u32::from_be_bytes(*b"wght")
        );
    }

    #[test]
    fn backend_boundary_accepts_metadata_without_changing_bootstrap_geometry() {
        let fallback = FontFallbackChain::default();
        let run = shaping_runs("abc", &fallback)[0];
        let face = fallback.face(run.face).unwrap();
        let backend = FixedTextShaper::default();
        let baseline = backend.shape_run("abc", &ShapingRequest::bootstrap("abc", run), face);
        let mut configured = ShapingRequest::bootstrap("abc", run);
        configured.language = LanguageTag::new("en");
        configured.features.push(OpenTypeFeature {
            tag: OpenTypeTag::from_bytes(*b"kern"),
            value: 1,
        });
        configured.variations.push(VariationCoordinate {
            axis: OpenTypeTag::from_bytes(*b"wght"),
            value: 700.0,
        });
        let shaped = backend.shape_run("abc", &configured, face);
        assert_eq!(baseline, shaped);
    }

    #[test]
    fn common_ascii_punctuation_and_digits_do_not_create_script_boundaries() {
        let fallback = FontFallbackChain::default();
        let run = TextRun::with_fallback("abc-123 Привет".into(), &fallback);
        let requests = run.shaping_requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].script, ShapingScript::Latin);
        assert_eq!(requests[1].script, ShapingScript::Cyrillic);
        assert_eq!(requests[0].run.range.end, requests[1].run.range.start);
    }
}

#[cfg(test)]
mod audit_font_fallback_tests {
    use super::*;

    #[test]
    fn empty_font_fallback_chain_is_non_panicking() {
        let chain = FontFallbackChain { faces: Vec::new() };
        assert!(font_runs("Rarog", &chain).is_empty());
        assert!(shaping_runs("Rarog", &chain).is_empty());
    }
}
