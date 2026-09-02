# ADR 0014 — Standards HTML parser adapter

## Status

Accepted for R1 — Flame.

## Decision

Rarog will use `html5ever` behind a Rarog-owned adapter for standards-oriented HTML tokenization and tree construction.

The adapter owns the integration boundary. `html5ever` node, atom, tendril and tree-sink types do not become public Rarog API and do not replace `rarog-dom` as the engine DOM model.

The first R1 slice exposes the standards parser alongside the retained bootstrap parser. The default engine parser is switched only after differential and focused WPT coverage demonstrates that the standards path preserves Rarog resource and no-panic contracts.

## Rationale

HTML tokenization and tree-building are tightly state-coupled by the HTML standard. Reusing a mature implementation avoids maintaining a second incomplete tokenizer while keeping DOM ownership, mutation semantics and rendering architecture inside Rarog.

`html5ever 0.39` is MIT OR Apache-2.0 and has an MSRV below Rarog's Rust 1.85 contract.

## Constraints

- conversion into `rarog-dom` is iterative;
- unsupported non-rendering node kinds remain inside the adapter until Rarog defines their DOM representation;
- parser diagnostics are translated into Rarog-owned diagnostics;
- dependency types must not leak into public engine interfaces;
- the bootstrap parser remains available during measured migration only, not as a permanent compatibility fallback.
