# ADR-0025: HTML streaming and standards parser boundary

## Status

Accepted.

## Context

The R0 bootstrap parser proves the DOM-to-rendering pipeline but is not an HTML standards implementation. Before replacing it, Rarog needs stable ownership boundaries for streaming input, diagnostics, source locations and the eventual tokenizer/tree-builder implementation. Those contracts must not expose a third-party parser's internal node model or force networking, encoding, DOM and parser lifetimes into one component.

## Decision

`rarog-html` owns a decoded character-stream boundary. `StreamingInput` accepts UTF-8 `str` chunks, can be closed explicitly and is parsed only after closure in R0. Source spans are UTF-8 byte offsets in that decoded stream. Network byte transport, content-encoding handling and HTML encoding detection/decoding remain upstream concerns until a standards implementation gives a reason to move part of that responsibility behind the parser adapter.

Recoverable syntax problems produce deterministic `ParseDiagnostic` values with a stable code, severity, source span and message. Parser/API contract failures that prevent parsing from starting or completing use `Result::Err`. The convenience `parse(&str) -> Document` entry point remains for current engine callers; `parse_with_diagnostics` and `parse_stream` expose the richer contract.

The R0 bootstrap parser may buffer all chunks until end of input. This establishes streaming ownership without claiming incremental tokenization or speculative tree construction.

R1 replaces the bootstrap algorithm behind these entry points with a WHATWG-oriented tokenizer and tree builder through an adapter. The tokenizer, tree builder and DOM sink stay separable. A mature Rust implementation may be used initially if it meets compatibility and maintenance requirements, but its token/node types must not become public Rarog engine types. The adapter must preserve Rarog `Document`, namespace and diagnostic boundaries so the implementation remains replaceable.

## Consequences

- Chunk boundaries cannot change the deterministic DOM or diagnostic result for equivalent decoded input.
- HTML syntax errors are observable without turning ordinary recovery into fatal engine errors.
- The current renderer keeps using `parse(&str)` without absorbing parser-specific types.
- R0 does not claim incremental HTML parsing, encoding sniffing, WHATWG tokenizer compliance or tree-builder compliance.
- R1 can introduce a standards-oriented parser without changing DOM, layout or paint identities.
- Parser conformance will be measured with standards tests and real-Web cases rather than inferred from the adapter choice.
