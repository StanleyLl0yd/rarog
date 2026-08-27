# ADR-0010: Flow-aware local reflow

## Status

Accepted for R0.

## Context

Rarog already reuses the Layout Tree for geometry changes. Horizontal or otherwise footprint-safe changes can relayout one Fragment subtree, but a change to height or vertical margin, border or padding can move following siblings and can change the natural height of ancestors.

Rebuilding the entire Fragment Tree is correct but defeats the purpose of the next incremental-rendering experiment.

## Decision

For the R0 block-flow model, vertical-footprint invalidation is propagated to the root block-flow child containing the earliest dirty node. Fragment children before that root-flow child are retained. That child and all following root-flow siblings are rebuilt from the retained Layout Tree using the previous sibling boundary as the new flow cursor.

The engine reports this path as `FlowRelayout`.

If the dirty nodes cannot be mapped safely to the current root flow, the engine falls back to whole-Fragment-Tree `GeometryRelayout`. Structural, text and display-membership changes continue to use `FullRebuild`.

Paint remains derived from the resulting Fragment Tree. Damage is computed against the previous display list and only damaged framebuffer regions are rerasterized.

## Consequences

This establishes ancestor/sibling-aware propagation for the current vertical block-flow bootstrap while preserving unaffected prefixes and the Layout Tree. It is intentionally not a general CSS reflow algorithm: nested formatting-context-local propagation, margin collapsing, inline fragmentation, floats, positioning, writing modes and fragmentation remain future work.

Correctness is checked against a deterministic full render for the same final DOM/style state.
