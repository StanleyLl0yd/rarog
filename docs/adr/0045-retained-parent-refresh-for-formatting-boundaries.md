# ADR-0045 — Retained parent refresh for formatting boundaries

Status: Accepted

## Context

R1 incremental rendering already refreshes retained layout subtrees for DOM insertions, reparenting and detach, and revalidates stylesheet-source changes without rebuilding the complete Layout Tree. Style changes that alter formatting membership were still treated differently: `display:none` visibility changes, block/inline role changes and `display:flow-root` BFC changes forced `FullRebuild`.

Those changes cannot be handled by patching one retained layout node in place because they may add or remove that node from its parent's layout children or change how siblings participate in inline/block formatting. However, the existing retained structural refresh boundary can rebuild the nearest represented parent while preserving stable `LayoutNodeId` values for unaffected and surviving descendants.

## Decision

Treat formatting-boundary style changes as retained structural invalidation.

When a connected style candidate changes visibility membership, block/inline role or BFC establishment, the engine finds the nearest DOM parent represented in the retained Layout Tree and schedules that parent for `refresh_layout_subtrees`. A connected element that was absent from retained layout but becomes visible uses the same parent-refresh path; an element that remains `display:none` requires no layout work.

Stylesheet-source revalidation scans connected elements for the same formatting-boundary changes before global retained style comparison so hidden-to-visible nodes are not missed merely because they had no previous layout node.

Formatting roots are minimized, refreshed with the new `StyleSet`, merged into the flow-relayout dirty set, and then rebuilt through `relayout_fragment_flow`. Style/text work covered by a refreshed formatting root is discarded because the subtree refresh has already consumed it.

If no retained parent can safely cover a required formatting-boundary change, the engine keeps the existing `FullRebuild` fallback.

## Consequences

Ordinary `display:block ↔ none`, `display:block ↔ inline`, and `display:block/inline ↔ flow-root` changes no longer require rebuilding the complete Layout Tree when a retained parent exists. Parent and unaffected subtree layout identities remain stable, the display-list suffix can still be retained, and framebuffer output is verified against a fresh render.

This does not yet remove the separate conservative fallback for geometry-changing style updates mixed with text mutations or geometry changes applied directly to already-fragmented inline owners; those remain follow-up incremental-breadth work.
