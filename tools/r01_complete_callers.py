from pathlib import Path


def replace_all(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found == 0:
        raise SystemExit(f"{path}: pattern not found: {old[:100]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


engine = "crates/rarog-engine/src/lib.rs"
replace_all(
    engine,
    "let report = session.update();",
    "let report = session.update().expect(\"incremental update succeeds\");",
)

correctness = "crates/rarog-engine/tests/r01_correctness.rs"
replace_all(
    correctness,
    "session.update().mode",
    "session.update().expect(\"incremental update succeeds\").mode",
)
replace_all(
    correctness,
    "structural.update().mode",
    "structural.update().expect(\"incremental update succeeds\").mode",
)

bench = "crates/rarog-engine/examples/r0_bench.rs"
replace_all(
    bench,
    "let report = session.update();",
    "let report = session.update().expect(\"benchmark update succeeds\");",
)

embedder = "crates/rarog-engine/src/embedder.rs"
text = Path(embedder).read_text(encoding="utf-8")
old = """                    max_document_source_bytes: 1,
                    max_viewport_pixels: MAX_FRAMEBUFFER_PIXELS + 1,
                })"""
new = """                    max_document_source_bytes: 1,
                    max_viewport_pixels: MAX_FRAMEBUFFER_PIXELS + 1,
                    ..ResourceBudget::default()
                })"""
if old not in text:
    raise SystemExit("embedder invalid-budget fixture pattern missing")
Path(embedder).write_text(text.replace(old, new), encoding="utf-8")

p1 = "crates/rarog-engine/tests/p1_exit.rs"
text = Path(p1).read_text(encoding="utf-8")
old = """            max_document_source_bytes: 32,
            max_viewport_pixels: 20_000,
        })"""
new = """            max_document_source_bytes: 32,
            max_viewport_pixels: 20_000,
            ..ResourceBudget::default()
        })"""
if old not in text:
    raise SystemExit("P1 budget fixture pattern missing")
Path(p1).write_text(text.replace(old, new), encoding="utf-8")
