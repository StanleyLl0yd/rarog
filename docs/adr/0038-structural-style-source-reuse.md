# ADR-0038: Structural rebuilds may reuse unchanged stylesheet sources

## Status

Accepted for R1 — Flame.

## Context

Structural DOM mutations currently require rebuilding Layout and Fragments for correctness, and historically that fallback also rebuilt the `StyleSet` unconditionally. Most ordinary child insertions and reparentings do not change inline `<style>` source text or connected stylesheet-source membership, so reparsing all stylesheet sources is unnecessary work.

## Decision

Separate the “full layout rebuild required” decision from the “stylesheet sources changed” decision.

An ordinary structural mutation may still use `IncrementalMode::FullRebuild` while reusing the existing `StyleSet` if no connected `<style>` source is introduced, removed, moved across a style-source boundary, or mutated.

`CharacterData` within `<style>`, inserting a connected subtree that contains `<style>`, and moves involving style-source containers continue to rebuild `StyleSet`.

Expose `IncrementalReport::styles_rebuilt` so tests and embedders can observe whether stylesheet-source reconstruction actually occurred.

## Consequences

- Structural layout correctness remains conservative.
- Ordinary DOM insertions avoid reparsing unchanged stylesheet sources.
- Style-source mutations still force fresh `StyleSet` construction.
- Some ambiguous reparent cases may over-invalidate safely until mutation records carry prior connectivity or source-membership information.

## Invariants

1. Reusing `StyleSet` must not change computed-style or framebuffer output relative to a fresh render.
2. Style-source uncertainty falls back to rebuilding `StyleSet`.
3. Layout fallback classification is independent from stylesheet-source freshness.
4. Resource and CSS rule limits are validated in either path.
