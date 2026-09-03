# Fuzzing

The fuzz package is intentionally excluded from the main Cargo workspace so product MSRV and release dependency surfaces do not inherit fuzz-only tooling.

Targets:

- `html_parse` feeds arbitrary UTF-8 HTML into the standards-oriented parser adapter and asserts that the resulting DOM preserves native invariants.
- `css_stylesheet` feeds arbitrary UTF-8 CSS into stylesheet parsing so malformed selector/declaration input must remain controlled and non-panicking.
- `render_html` feeds arbitrary UTF-8 HTML through the full render boundary with a small viewport. Any controlled `RenderError` is acceptable; a panic, abort, invariant failure, or memory-safety failure is not.

Typical local commands:

```text
cargo install cargo-fuzz --locked
cargo fuzz run html_parse -- -max_len=1048576
cargo fuzz run css_stylesheet -- -max_len=1048576
cargo fuzz run render_html -- -max_len=1048576
```

Keep crash artifacts produced by fuzzing for regression work, but do not treat a bounded fuzz run as proof of parser or renderer correctness. Reproduced crashes must become deterministic unit/integration tests before the fix is considered complete.
