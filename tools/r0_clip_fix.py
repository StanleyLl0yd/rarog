from pathlib import Path

path = Path('crates/rarog-paint/src/lib.rs')
text = path.read_text().replace('Size::new(8.0, 8.0)', 'Size { width: 8.0, height: 8.0 }')
path.write_text(text)
