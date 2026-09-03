# ADR-0048 — Standards WebIDL parser adapter

Status: Accepted

## Context

ADR-0047 established a Rarog-owned WebIDL IR and `WebIdlFrontend` boundary before selecting a concrete parser. R2 — Flight now needs a grammar-oriented parser implementation while preserving parser replaceability, Rust 1.85 support and the rule that dependency AST types never become downstream DOM/script contracts.

The normalized IR intentionally represents only the subset required for the first binding slices. Feeding a broader parser AST through this boundary must therefore fail explicitly whenever a construct cannot be represented without loss.

## Decision

Use `weedle2` 5.0.0 as the first standards-oriented parser dependency inside `rarog-webidl`, imported privately as `weedle` and pinned exactly in the crate manifest.

`StandardsWebIdlFrontend` implements the existing Rarog-owned `WebIdlFrontend` trait. The adapter calls `weedle::Definitions::parse` directly instead of the dependency's convenience `parse` function so redundant/unconsumed input is converted into a Rarog `WebIdlError` rather than reaching the dependency's assertion path. Parser failures and unconsumed input are mapped into owned frontend errors with source spans.

The first normalized subset includes:

- interfaces, partial interfaces, interface mixins and partial interface mixins;
- dictionaries and partial dictionaries;
- enums, typedefs and includes statements;
- regular attributes and operations, including representable readonly/static metadata;
- required dictionary members and optional/variadic operation arguments;
- the existing Rarog scalar, string, named, sequence, frozen-array, promise, record, union and nullable type shapes.

Constructs that the current Rarog IR cannot represent are rejected with `WebIdlErrorKind::UnsupportedDefinition`. This includes extended attributes, default values, callbacks, callback interfaces, namespaces, legacy implements statements, constants, constructors, special/stringifier operations and collection members such as iterable/maplike/setlike. Unsupported data is never silently dropped.

All dependency AST values are converted into owned Rarog types before `parse` returns. No `weedle2` or `nom` type appears in a public `rarog-webidl` API.

## Consequences

- R2 has a real WebIDL grammar parser while parser replacement remains local to `rarog-webidl`.
- The dependency graph gains the pinned parser and its small parsing support chain; `Cargo.lock` records the exact resolved versions.
- Broader WebIDL support must first extend the Rarog-owned IR deliberately, then extend normalization, rather than exposing vendor AST details.
- Malformed or unsupported input fails closed at the frontend boundary.
- The adapter and dependency chain are verified on Windows, Linux and Rust 1.85 before merge.
