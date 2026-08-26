# ADR-0001: Rust-first implementation

**Status:** Accepted

## Decision

Engine-owned components are implemented in safe Rust by default. External mature components may be integrated behind adapters where implementing them ourselves would materially delay Web compatibility.

## Consequences

- memory-safety bugs are reduced at architectural level;
- `unsafe` requires an explicit audited boundary later;
- Rarog is not marketed as "100% Rust" because JS engines, codecs and system libraries may not be Rust.
