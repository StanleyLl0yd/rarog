from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
text = path.read_text()
text = text.replace("&new_styles", "new_styles")
path.write_text(text)
