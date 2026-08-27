from pathlib import Path
p=Path('crates/rarog-layout/src/lib.rs')
s=p.read_text()
old='''fn shaping_script_for_character(character: char) -> Option<ShapingScript> {\n    let code = character as u32;\n    if is_extended_pictographic(character) || is_regional_indicator(character) {\n        Some(ShapingScript::Emoji)\n    } else if matches!(code, 0x0041..=0x024f) {\n        Some(ShapingScript::Latin)\n'''
new='''fn shaping_script_for_character(character: char) -> Option<ShapingScript> {\n    let code = character as u32;\n    if is_extended_pictographic(character) || is_regional_indicator(character) {\n        Some(ShapingScript::Emoji)\n    } else if is_common_font_character(character) || character.is_ascii_digit() {\n        None\n    } else if matches!(code, 0x0041..=0x024f) {\n        Some(ShapingScript::Latin)\n'''
if old not in s: raise SystemExit('classification anchor missing')
s=s.replace(old,new,1)
s=s.replace('''    } else if is_common_font_character(character) || is_grapheme_extend(character) {\n        None\n    } else {\n''','''    } else if is_grapheme_extend(character) {\n        None\n    } else {\n''',1)
end=s.rfind('\n}')
test=r'''

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
'''
s=s[:end]+test+s[end:]
p.write_text(s)
