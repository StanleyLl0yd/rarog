from pathlib import Path

path = Path('crates/rarog-layout/src/lib.rs')
text = path.read_text()
old = '''        let face = chain
            .select_face_for_range(text, range)
            .or_else(|| chain.faces.last().map(|face| face.id))
            .expect("font fallback chain must contain at least one face");
'''
new = '''        let characters = text.chars().collect::<Vec<_>>();
        let common = characters[range.start..range.end]
            .iter()
            .copied()
            .all(is_common_font_character);
        let inherited = common && !runs.is_empty();
        let face = if inherited {
            runs.last().map(|run| run.face)
        } else {
            chain.select_face_for_range(text, range)
        }
        .or_else(|| chain.faces.last().map(|face| face.id))
        .expect("font fallback chain must contain at least one face");
'''
if old not in text:
    raise SystemExit('font selection block not found')
text = text.replace(old, new, 1)
marker = '''pub fn font_runs(text: &str, chain: &FontFallbackChain) -> Vec<FontRun> {
'''
helper = '''fn is_common_font_character(character: char) -> bool {
    let code = character as u32;
    is_grapheme_extend(character)
        || character.is_whitespace()
        || character.is_ascii_punctuation()
        || matches!(code, 0x2000..=0x206f)
}

'''
if marker not in text:
    raise SystemExit('font_runs marker not found')
text = text.replace(marker, helper + marker, 1)
path.write_text(text)
