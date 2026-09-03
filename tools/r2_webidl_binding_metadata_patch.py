from pathlib import Path

path = Path("crates/rarog-webidl/src/lib.rs")
text = path.read_text()

prefix = "mod weedle_frontend;\n\npub use weedle_frontend::StandardsWebIdlFrontend;\n"
replacement = (
    "mod binding;\n"
    "mod weedle_frontend;\n\n"
    "pub use binding::{BindingMetadata, build_binding_metadata};\n"
    "pub use weedle_frontend::StandardsWebIdlFrontend;\n"
)
if prefix not in text:
    raise SystemExit("rarog-webidl module prefix missing")
text = text.replace(prefix, replacement, 1)

old = "    Frontend,\n    UnsupportedDefinition,\n"
new = "    Frontend,\n    UnsupportedDefinition,\n    Validation,\n"
if old not in text:
    raise SystemExit("WebIdlErrorKind variants missing")
text = text.replace(old, new, 1)

path.write_text(text)
