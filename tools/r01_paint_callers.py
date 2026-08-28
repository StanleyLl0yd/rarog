from pathlib import Path

replacements = {
    "crates/rarog-engine/src/lib.rs": [
        ("output.display_list.commands.is_empty()", "output.display_list.is_empty()"),
    ],
    "crates/rarog-engine/tests/p1_exit.rs": [
        ("frame.display_list.commands.len()", "frame.display_list.len()"),
    ],
    "crates/rarog-shell/src/main.rs": [
        ("rendered.display_list.commands.len()", "rendered.display_list.len()"),
    ],
}

for path, items in replacements.items():
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    for old, new in items:
        if old not in text:
            raise SystemExit(f"{path}: missing pattern {old!r}")
        text = text.replace(old, new)
    file.write_text(text, encoding="utf-8")
