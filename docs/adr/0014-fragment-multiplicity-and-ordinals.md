# ADR-0014: Fragment multiplicity and ordinals

## Status

Accepted for R0.

## Context

A Web layout object cannot be assumed to produce exactly one painted fragment. Inline layout, line breaking, pagination, columns, and other fragmentation modes can produce multiple fragments from one layout source. Rarog therefore needs to prove one-to-many layout-to-fragment identity before standards-oriented text and formatting contexts are introduced.

## Decision

A `LayoutNode` may emit multiple `Fragment` values. Every fragment keeps its ephemeral `FragmentId` for snapshot-local allocation identity and also carries a `FragmentOrdinal` that identifies its position within the fragments produced by the same source layout node.

Display-item identity uses the stable source identity plus `FragmentOrdinal` plus paint slot rather than the ephemeral `FragmentId`. This keeps multiple fragments distinct while avoiding unnecessary retained-paint churn caused only by snapshot allocation order.

The R0 proof case splits bootstrap fixed-advance text into multiple vertically arranged fragments when the containing block is narrow. This is only an architectural multiplicity test; it is not CSS inline formatting or standards-compliant line breaking.

Incremental subtree/flow paths remain conservative around fragmented sources. If a retained local replacement cannot prove that the fragment multiplicity and structural range are safe, it returns failure and the engine uses an existing broader relayout/rebuild fallback.

## Consequences

Future line boxes, shaping, bidi, pagination, columns, and other fragmentation work can build on a one-to-many fragment model without changing the fundamental layout/paint identity contract again. `FragmentId` remains disposable derived state; DOM and layout ownership never depend on it.
