from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text().replace(
    "FixedAdvanceLineBreaker::default()",
    "FixedAdvanceLineBreaker",
)
path.write_text(text)
