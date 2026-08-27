from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text().replace(
    "let breaker = FixedAdvanceLineBreaker;",
    "let breaker = UnicodeLineBreaker;",
)
text = text.replace(
    ".map(|value| value.index)\n                    .last();",
    ".map(|value| value.index)\n                    .next_back();",
)
path.write_text(text)
