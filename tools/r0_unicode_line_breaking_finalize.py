from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text().replace(
    "let breaker = FixedAdvanceLineBreaker;",
    "let breaker = UnicodeLineBreaker;",
)
path.write_text(text)
