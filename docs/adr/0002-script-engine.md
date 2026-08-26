# ADR-0002: Do not build a JavaScript engine for Rarog 1.x

**Status:** Accepted

## Decision

Use a replaceable script-runtime interface and target SpiderMonkey as the first production JavaScript/Wasm backend.

## Reason

Building DOM/CSS/layout/rendering and a competitive JS VM/JIT/GC simultaneously would multiply project risk without creating the primary Rarog advantage.
