from pathlib import Path

engine = Path("crates/rarog-engine/src/lib.rs")
text = engine.read_text().replace(
    "2_598_378_446_485_377_948",
    "16_985_642_107_972_200_629",
    1,
)
engine.write_text(text)

adr = Path("docs/adr/0015-line-boxes-and-text-ranges.md")
adr.write_text("""# ADR-0015: Line boxes and text ranges

## Status

Accepted.

## Context

R0 fragmentation established that one layout node may emit multiple fragments, but text fragments still lacked an explicit mapping back to the source text and line geometry was implicit in fragment rectangles.

## Decision

Text fragments carry a source-character `TextRange` and a `LineBox`. Line-breaking policy is isolated behind the `LineBreaker` trait. The R0 implementation uses deterministic fixed-width character advances only as a bootstrap policy.

`FragmentOrdinal` remains the stable per-source fragment identity used by paint. Text ranges describe source coverage and do not replace fragment identity.

## Consequences

Future shaping, font metrics, bidi processing, and standards-compliant line breaking can replace the bootstrap breaker without changing retained-paint identity contracts. Incremental layout can reason about explicit source ranges and line geometry rather than reconstructing them from fragment order.
""")
