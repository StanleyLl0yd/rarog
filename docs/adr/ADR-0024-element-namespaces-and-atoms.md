# ADR-0024: Element namespaces and atom strategy

## Status

Accepted.

## Context

R0 originally stored an element name as a plain `String` and implicitly treated every element as HTML. That is sufficient for the first bootstrap fixture but is the wrong ownership boundary for a Web DOM that will later represent HTML, SVG, MathML and namespaced content. Frequently repeated engine names also need a stable semantic type before selector, parser and WebIDL work make string allocation policy harder to change.

## Decision

Every `ElementData` stores an explicit `Namespace` plus an `Atom` local name. R0 defines built-in HTML, SVG and MathML namespace variants and an `Other(Atom)` escape hatch without claiming namespace-aware HTML parsing. The bootstrap HTML parser creates HTML elements only.

`Atom` uses immutable shared `Arc<str>` storage in R0. Clones share the same allocation, but independently-created equal atoms are not required to be pointer-identical. If measurements justify canonical interning, it will be document/process scoped and implemented behind the atom boundary rather than through a process-global immortal table.

Text-node data and attribute values are not atomized. Attribute-name atomization and namespace-aware attributes may be introduced later when the standards parser and selector model require them.

## Consequences

- DOM element namespace is no longer implicit in a tag-name string.
- Existing HTML snapshots keep their previous spelling; non-HTML snapshots include a namespace prefix for deterministic diagnostics.
- Atom cloning is cheap and gives later parser/style code a replaceable string-storage boundary.
- There is no global interner lifetime shared across sites or processes.
- This ADR does not implement HTML namespace switching, foreign-content parsing, namespaced CSS selectors or SVG/MathML layout semantics.
