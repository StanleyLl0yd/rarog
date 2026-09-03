from pathlib import Path

lib = Path("crates/rarog-webidl/src/lib.rs")
text = lib.read_text()
module = "mod weedle_frontend;\n\npub use weedle_frontend::StandardsWebIdlFrontend;\n\n"
if module not in text:
    text = module + text
lib.write_text(text)

manifest = Path("crates/rarog-webidl/Cargo.toml")
text = manifest.read_text()
dependency = 'weedle = { package = "weedle2", version = "=5.0.0" }'
if dependency not in text:
    marker = "\n[lib]\n"
    if marker not in text:
        raise SystemExit("rarog-webidl manifest lib marker missing")
    text = text.replace(marker, f"\n[dependencies]\n{dependency}\n{marker}", 1)
manifest.write_text(text)

adapter = Path("crates/rarog-webidl/src/weedle_frontend.rs")
text = adapter.read_text()
text = text.replace(
    'snapshot.contains("operation|5:reset|undefined|true")',
    'snapshot.contains("operation||5:reset|undefined|true")',
)
adapter.write_text(text)
