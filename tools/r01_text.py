from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:120]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


layout = "crates/rarog-layout/src/lib.rs"
replace(
    layout,
    """    pub fn select_face_for_range(&self, text: &str, range: TextRange) -> Option<FontFaceId> {
        let characters = text.chars().collect::<Vec<_>>();
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
}""",
    """    pub fn select_face_for_range(&self, text: &str, range: TextRange) -> Option<FontFaceId> {
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
            .find(|face| slice.iter().copied().all(|character| face.covers(character)))
            .map(|face| face.id)
    }
}""",
)
replace(
    layout,
    """pub fn font_runs(text: &str, chain: &FontFallbackChain) -> Vec<FontRun> {
    let boundaries = grapheme_boundaries(text);
    if boundaries.len() < 2 {
        return Vec::new();
    }

    let mut runs: Vec<FontRun> = Vec::new();
    for window in boundaries.windows(2) {
        let range = TextRange::new(window[0], window[1]);
        let characters = text.chars().collect::<Vec<_>>();
        let common = characters[range.start..range.end]
            .iter()
            .copied()
            .all(is_common_font_character);
        let inherited = common && !runs.is_empty();
        let face = if inherited {
            runs.last().map(|run| run.face)
        } else {
            chain.select_face_for_range(text, range)
        }""",
    """pub fn font_runs(text: &str, chain: &FontFallbackChain) -> Vec<FontRun> {
    let characters = text.chars().collect::<Vec<_>>();
    let boundaries = grapheme_boundaries_for_characters(&characters);
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
            chain.select_face_for_characters(&characters, range)
        }""",
)
replace(
    layout,
    """fn shaping_runs_for_font_runs(text: &str, fonts: &[FontRun]) -> Vec<ShapingRun> {
    let bidi = bidi_runs(text);
    let mut runs: Vec<ShapingRun> = Vec::new();""",
    """fn shaping_runs_for_font_runs(text: &str, fonts: &[FontRun]) -> Vec<ShapingRun> {
    let bidi = bidi_runs(text);
    let boundaries = grapheme_boundaries(text);
    let mut runs: Vec<ShapingRun> = Vec::new();""",
)
replace(
    layout,
    """        if start < end {
            debug_assert!(is_grapheme_boundary(text, start));
            debug_assert!(is_grapheme_boundary(text, end));""",
    """        if start < end {
            debug_assert!(boundaries.binary_search(&start).is_ok());
            debug_assert!(boundaries.binary_search(&end).is_ok());""",
)
replace(
    layout,
    """fn shaping_requests_for_runs(text: &str, runs: &[ShapingRun]) -> Vec<ShapingRequest> {
    let boundaries = grapheme_boundaries(text);
    let mut requests = Vec::new();""",
    """fn shaping_requests_for_runs(text: &str, runs: &[ShapingRun]) -> Vec<ShapingRequest> {
    let characters = text.chars().collect::<Vec<_>>();
    let boundaries = grapheme_boundaries_for_characters(&characters);
    let mut requests = Vec::new();""",
)
replace(
    layout,
    """            let cluster_script =
                shaping_script_for_range(text, TextRange::new(cluster_start, cluster_end));""",
    """            let cluster_script = shaping_script_for_characters(
                &characters,
                TextRange::new(cluster_start, cluster_end),
            );""",
)
replace(
    layout,
    """                    let mut request = ShapingRequest::bootstrap(text, request_run);
                    request.script = script;
                    requests.push(request);""",
    """                    requests.push(shaping_request(request_run, script));""",
)
replace(
    layout,
    """            let mut request = ShapingRequest::bootstrap(text, request_run);
            if let Some(script) = current_script {
                request.script = script;
            }
            requests.push(request);""",
    """            let script = current_script.unwrap_or_else(|| {
                shaping_script_for_characters(&characters, request_run.range)
            });
            requests.push(shaping_request(request_run, script));""",
)
replace(
    layout,
    """pub fn shaping_script_for_range(text: &str, range: TextRange) -> ShapingScript {
    let characters = text.chars().collect::<Vec<_>>();
    let Some(slice) = characters.get(range.start..range.end) else {
        return ShapingScript::Unknown;
    };
    slice
        .iter()
        .copied()
        .find_map(shaping_script_for_character)
        .unwrap_or(ShapingScript::Common)
}""",
    """fn shaping_request(run: ShapingRun, script: ShapingScript) -> ShapingRequest {
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
}""",
)
replace(
    layout,
    """pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut boundaries = vec![0];
    for index in 1..characters.len() {
        let previous = characters[index - 1];
        let current = characters[index];
        let previous_previous = index.checked_sub(2).map(|value| characters[value]);
        let preceding_regional_indicators = characters[..index]
            .iter()
            .rev()
            .take_while(|character| is_regional_indicator(**character))
            .count();

        let no_break = (previous == '\\r' && current == '\\n')
            || is_grapheme_extend(current)
            || previous == '\\u{200d}'
            || current == '\\u{200d}'
            || (is_regional_indicator(previous)
                && is_regional_indicator(current)
                && preceding_regional_indicators % 2 == 1)
            || (previous_previous == Some('\\u{200d}') && is_extended_pictographic(current));

        if !no_break {
            boundaries.push(index);
        }
    }

    boundaries.push(characters.len());
    boundaries.dedup();
    boundaries
}""",
    """pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let characters = text.chars().collect::<Vec<_>>();
    grapheme_boundaries_for_characters(&characters)
}

fn grapheme_boundaries_for_characters(characters: &[char]) -> Vec<usize> {
    let mut boundaries = vec![0];
    if characters.is_empty() {
        return boundaries;
    }

    let mut preceding_regional_indicators = usize::from(is_regional_indicator(characters[0]));
    for index in 1..characters.len() {
        let previous = characters[index - 1];
        let current = characters[index];
        let previous_previous = index.checked_sub(2).map(|value| characters[value]);

        let no_break = (previous == '\\r' && current == '\\n')
            || is_grapheme_extend(current)
            || previous == '\\u{200d}'
            || current == '\\u{200d}'
            || (is_regional_indicator(previous)
                && is_regional_indicator(current)
                && preceding_regional_indicators % 2 == 1)
            || (previous_previous == Some('\\u{200d}') && is_extended_pictographic(current));

        if !no_break {
            boundaries.push(index);
        }

        preceding_regional_indicators = if is_regional_indicator(current) {
            if is_regional_indicator(previous) {
                preceding_regional_indicators.saturating_add(1)
            } else {
                1
            }
        } else {
            0
        };
    }

    boundaries.push(characters.len());
    boundaries.dedup();
    boundaries
}""",
)
replace(
    layout,
    """pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut opportunities = Vec::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let boundary = index + 1;
        if is_mandatory_break(character) {
            if is_grapheme_boundary(text, boundary) {
                opportunities.push(BreakOpportunity {
                    index: boundary,
                    kind: BreakKind::Mandatory,
                });
            }
            continue;
        }
        let next = characters.get(boundary).copied();
        if is_grapheme_boundary(text, boundary)
            && (is_breakable_whitespace(character)
                || character == '-'
                || (is_cjk_ideograph(character) && next.is_some_and(is_cjk_ideograph)))
        {
            opportunities.push(BreakOpportunity {
                index: boundary,
                kind: BreakKind::Soft,
            });
        }
    }
    opportunities
}""",
    """pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    let characters = text.chars().collect::<Vec<_>>();
    let boundaries = grapheme_boundaries_for_characters(&characters);
    let mut is_boundary = vec![false; characters.len().saturating_add(1)];
    for boundary in boundaries {
        if let Some(value) = is_boundary.get_mut(boundary) {
            *value = true;
        }
    }

    let mut opportunities = Vec::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let boundary = index + 1;
        if is_mandatory_break(character) {
            if is_boundary[boundary] {
                opportunities.push(BreakOpportunity {
                    index: boundary,
                    kind: BreakKind::Mandatory,
                });
            }
            continue;
        }
        let next = characters.get(boundary).copied();
        if is_boundary[boundary]
            && (is_breakable_whitespace(character)
                || character == '-'
                || (is_cjk_ideograph(character) && next.is_some_and(is_cjk_ideograph)))
        {
            opportunities.push(BreakOpportunity {
                index: boundary,
                kind: BreakKind::Soft,
            });
        }
    }
    opportunities
}""",
)
replace(
    layout,
    """fn is_non_breaking_boundary(text: &str, index: usize) -> bool {
    let characters = text.chars().collect::<Vec<_>>();
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index);""",
    """fn is_non_breaking_boundary(characters: &[char], index: usize) -> bool {
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index);""",
)
replace(
    layout,
    """        let opportunities = unicode_break_opportunities(&run.text);
        let mut ranges = Vec::new();
        let mut line_start = 0;
        let mut last_soft = None;
        let mut width = 0.0;""",
    """        let characters = run.text.chars().collect::<Vec<_>>();
        let boundaries = grapheme_boundaries_for_characters(&characters);
        let opportunities = unicode_break_opportunities(&run.text);
        let mut prefix_advance = vec![0.0f32; characters.len().saturating_add(1)];
        for cluster in &run.shaped.clusters {
            prefix_advance[cluster.source.end] =
                prefix_advance[cluster.source.start] + cluster.advance;
        }
        let mut ranges = Vec::new();
        let mut line_start = 0;
        let mut last_soft = None;
        let mut width = 0.0;""",
)
replace(
    layout,
    """            let opportunity = opportunities
                .iter()
                .find(|opportunity| opportunity.index == boundary)
                .copied();""",
    """            let opportunity = opportunities
                .binary_search_by_key(&boundary, |opportunity| opportunity.index)
                .ok()
                .map(|index| opportunities[index]);""",
)
replace(
    layout,
    """                    (emergency > line_start
                        && is_grapheme_boundary(&run.text, emergency)
                        && !is_non_breaking_boundary(&run.text, emergency))
                    .then_some(emergency)""",
    """                    (emergency > line_start
                        && boundaries.binary_search(&emergency).is_ok()
                        && !is_non_breaking_boundary(&characters, emergency))
                    .then_some(emergency)""",
)
replace(
    layout,
    """                    width = run.advance_for_range(TextRange::new(line_start, boundary));
                    last_soft = opportunities
                        .iter()
                        .filter(|value| {
                            value.kind == BreakKind::Soft
                                && value.index > line_start
                                && value.index < boundary
                        })
                        .map(|value| value.index)
                        .next_back();""",
    """                    width = prefix_advance[boundary] - prefix_advance[line_start];
                    let end = opportunities.partition_point(|value| value.index < boundary);
                    last_soft = opportunities[..end]
                        .iter()
                        .rev()
                        .find(|value| value.kind == BreakKind::Soft && value.index > line_start)
                        .map(|value| value.index);""",
)
# Add a long-text regression that exercises the former repeated-boundary hot paths.
marker = """    #[test]
    fn crlf_is_one_grapheme_cluster_and_one_mandatory_boundary() {"""
text = Path(layout).read_text(encoding="utf-8")
if marker not in text:
    raise SystemExit("layout long-text test marker missing")
test = """    #[test]
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

"""
Path(layout).write_text(text.replace(marker, test + marker), encoding="utf-8")
