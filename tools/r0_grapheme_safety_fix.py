from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text()
text = text.replace("    let mut regional_run = 0usize;\n\n", "")
text = text.replace("        let previous_previous = index.checked_sub(2).map(|value| characters[value]);\n\n", "        let previous_previous = index.checked_sub(2).map(|value| characters[value]);\n        let preceding_regional_indicators = characters[..index]\n            .iter()\n            .rev()\n            .take_while(|character| is_regional_indicator(**character))\n            .count();\n\n", 1)
text = text.replace("                && regional_run % 2 == 1)", "                && preceding_regional_indicators % 2 == 1)", 1)
start = text.find("\n        if is_regional_indicator(current) {")
if start < 0:
    raise SystemExit("regional state block not found")
end = text.find("\n        }\n", start)
if end < 0:
    raise SystemExit("regional state block end not found")
end += len("\n        }\n")
text = text[:start] + text[end:]
path.write_text(text)
