# ADR-0035: Incremental character-data reflow

## Status

Accepted for R1 — Flame.

## Context

Rarog already retains Layout and Fragment trees across paint-only and geometry style changes, and can reflow a root block-flow suffix while preserving unaffected prefix fragments. Ordinary DOM `CharacterData` mutations still forced `FullRebuild`, even when the text node already had a stable LayoutNode and no stylesheet source changed.

## Decision

For an existing ordinary text node, `RenderSession::update` may refresh the retained LayoutNode's `TextRun`, recompute intrinsic sizes from that node through its retained ancestor chain, and route the node through the existing flow-aware fragment relayout path.

The retained LayoutNode and its ID remain stable. The fragment flow is rebuilt from the earliest affected root-flow child; unaffected prefix fragments retain their IDs. Paint damage is derived from the old and updated display lists and only damaged framebuffer regions are rasterized.

The first R1 slice remains conservative:
- character data inside a `<style>` element uses `FullRebuild`;
- structural mutations use `FullRebuild`;
- mixed style and text mutations in one update use `FullRebuild`;
- if the retained text LayoutNode cannot be found or refreshed, use `FullRebuild`.

Incremental correctness is checked against a fresh full render for the same final DOM.

## Consequences

- Common text edits no longer require rebuilding the entire Layout Tree.
- Text width and line-count changes reuse the existing flow-aware reflow algorithm.
- Layout identity can survive ordinary character-data edits.
- This does not yet make style/text mixed updates or structural DOM changes incremental.
- Production font integration remains orthogonal to this invalidation/reflow decision.

## Invariants

1. DOM mutation records remain layout-independent.
2. Incremental text refresh does not allocate replacement LayoutNode IDs.
3. Intrinsic sizes are recomputed along every retained ancestor whose text descendant changed.
4. Any unsafe or unmappable case falls back to deterministic full rebuild.
5. Incremental output must match a fresh full render.
