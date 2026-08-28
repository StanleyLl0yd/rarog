from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:120]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


cargo = "crates/rarog-layout/Cargo.toml"
replace(
    cargo,
    """rarog-types = { path = "../rarog-types" }
""",
    """rarog-types = { path = "../rarog-types" }
unicode-bidi = "0.3.18"
unicode-linebreak = "0.1.5"
unicode-script = "0.5.8"
unicode-segmentation = "1.13.3"
""",
)

layout = "crates/rarog-layout/src/lib.rs"
replace(
    layout,
    """use rarog_types::{Point, Rect, Size};
""",
    """use rarog_types::{Point, Rect, Size};
use unicode_bidi::BidiInfo;
use unicode_linebreak::{BreakOpportunity as UnicodeBreakOpportunity, linebreaks};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;
""",
)
# Use the standards grapheme iterator everywhere rather than the former char-slice helper.
text = Path(layout).read_text(encoding="utf-8")
text = text.replace("grapheme_boundaries_for_characters(&characters)", "grapheme_boundaries(text)")
text = text.replace("grapheme_boundaries_for_characters(&characters)", "grapheme_boundaries(&run.text)")
# The second global replacement above cannot distinguish contexts after the first replacement;
# correct the line-breaker context explicitly below if needed.
text = text.replace(
    "let boundaries = grapheme_boundaries(text);\n        let opportunities = unicode_break_opportunities(&run.text);",
    "let boundaries = grapheme_boundaries(&run.text);\n        let opportunities = unicode_break_opportunities(&run.text);",
)
Path(layout).write_text(text, encoding="utf-8")

replace(
    layout,
    """fn shaping_script_for_character(character: char) -> Option<ShapingScript> {
    let code = character as u32;
    if is_extended_pictographic(character) || is_regional_indicator(character) {
        Some(ShapingScript::Emoji)
    } else if is_common_font_character(character) || character.is_ascii_digit() {
        None
    } else if matches!(code, 0x0041..=0x024f) {
        Some(ShapingScript::Latin)
    } else if matches!(code, 0x0400..=0x052f) {
        Some(ShapingScript::Cyrillic)
    } else if matches!(code, 0x0590..=0x05ff) {
        Some(ShapingScript::Hebrew)
    } else if matches!(code, 0x0600..=0x08ff | 0xfb50..=0xfdff | 0xfe70..=0xfefc) {
        Some(ShapingScript::Arabic)
    } else if matches!(code, 0x2e80..=0x9fff | 0xf900..=0xfaff) {
        Some(ShapingScript::Han)
    } else if is_grapheme_extend(character) {
        None
    } else {
        Some(ShapingScript::Unknown)
    }
}""",
    """fn shaping_script_for_character(character: char) -> Option<ShapingScript> {
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
}""",
)
replace(
    layout,
    """pub fn paragraph_direction(text: &str) -> TextDirection {
    text.chars()
        .find_map(strong_direction)
        .unwrap_or(TextDirection::Ltr)
}

pub fn bidi_runs(text: &str) -> Vec<BidiRun> {
    let characters = text.chars().collect::<Vec<_>>();
    if characters.is_empty() {
        return Vec::new();
    }

    let base = paragraph_direction(text);
    let base_level = match base {
        TextDirection::Ltr => BidiLevel::new(0),
        TextDirection::Rtl => BidiLevel::new(1),
    };

    let mut resolved = Vec::with_capacity(characters.len());
    let mut previous = base;
    for character in characters.iter().copied() {
        let direction = strong_direction(character).unwrap_or(previous);
        resolved.push(direction);
        if strong_direction(character).is_some() {
            previous = direction;
        }
    }

    let mut runs = Vec::new();
    let mut start = 0usize;
    let mut current = resolved[0];
    for (index, direction) in resolved.iter().copied().enumerate().skip(1) {
        if direction != current {
            runs.push(BidiRun {
                range: TextRange::new(start, index),
                level: level_for_direction(base_level, current),
            });
            start = index;
            current = direction;
        }
    }
    runs.push(BidiRun {
        range: TextRange::new(start, characters.len()),
        level: level_for_direction(base_level, current),
    });
    runs
}""",
    """pub fn paragraph_direction(text: &str) -> TextDirection {
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
}""",
)
# Remove the bootstrap direction helpers that are now replaced by UAX #9 data.
replace(
    layout,
    """fn level_for_direction(base: BidiLevel, direction: TextDirection) -> BidiLevel {
    match (base.direction(), direction) {
        (TextDirection::Ltr, TextDirection::Ltr) => BidiLevel::new(0),
        (TextDirection::Ltr, TextDirection::Rtl) => BidiLevel::new(1),
        (TextDirection::Rtl, TextDirection::Rtl) => BidiLevel::new(1),
        (TextDirection::Rtl, TextDirection::Ltr) => BidiLevel::new(2),
    }
}

fn strong_direction(character: char) -> Option<TextDirection> {
    let code = character as u32;
    if matches!(code, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfefc) {
        Some(TextDirection::Rtl)
    } else if character.is_alphabetic() || character.is_ascii_digit() {
        Some(TextDirection::Ltr)
    } else {
        None
    }
}

""",
    """,
)
# Replace the hand-maintained grapheme subset with UAX #29 data.
start = Path(layout).read_text(encoding="utf-8")
old_start = start.index("pub fn grapheme_boundaries(text: &str) -> Vec<usize> {")
old_end = start.index("pub fn is_grapheme_boundary", old_start)
new_grapheme = """pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::new();
    let mut character_index = 0usize;
    boundaries.push(0);
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        character_index = character_index.saturating_add(grapheme.chars().count());
        boundaries.push(character_index);
    }
    boundaries
}

"""
Path(layout).write_text(start[:old_start] + new_grapheme + start[old_end:], encoding="utf-8")
# Replace the hand-maintained break subset with UAX #14 data, keeping Rarog's char-index contract.
text = Path(layout).read_text(encoding="utf-8")
old_start = text.index("pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {")
old_end = text.index("fn is_mandatory_break", old_start)
new_breaks = """pub fn unicode_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    let mut opportunities = Vec::new();
    let mut previous_byte = 0usize;
    let mut character_index = 0usize;
    let terminal_is_explicit_break = text.chars().last().is_some_and(is_mandatory_break);

    for (byte_index, opportunity) in linebreaks(text) {
        character_index = character_index
            .saturating_add(text[previous_byte..byte_index].chars().count());
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

"""
text = text[:old_start] + new_breaks + text[old_end:]
# The former helper name no longer exists after switching to UnicodeSegmentation.
text = text.replace("grapheme_boundaries_for_characters(&characters)", "grapheme_boundaries(&run.text)")
Path(layout).write_text(text, encoding="utf-8")
# Standards line breaking may expose additional legal opportunities; test the contract, not the old subset.
text = Path(layout).read_text(encoding="utf-8")
old = """    #[test]
    fn unicode_break_opportunities_cover_whitespace_hyphen_cjk_and_mandatory_breaks() {
        assert_eq!(
            unicode_break_opportunities("a b-c中日\\nq"),
            vec![
                BreakOpportunity {
                    index: 2,
                    kind: BreakKind::Soft
                },
                BreakOpportunity {
                    index: 4,
                    kind: BreakKind::Soft
                },
                BreakOpportunity {
                    index: 6,
                    kind: BreakKind::Soft
                },
                BreakOpportunity {
                    index: 8,
                    kind: BreakKind::Mandatory
                },
            ]
        );
    }
"""
new = """    #[test]
    fn unicode_break_opportunities_cover_whitespace_hyphen_cjk_and_mandatory_breaks() {
        let opportunities = unicode_break_opportunities("a b-c中日\\nq");
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
"""
if old not in text:
    raise SystemExit("unicode break regression pattern missing")
Path(layout).write_text(text.replace(old, new), encoding="utf-8")
