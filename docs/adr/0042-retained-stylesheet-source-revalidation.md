# ADR-0042 — Retained stylesheet-source revalidation

Status: Accepted

## Context

R1 incremental rendering already retains layout across ordinary attribute, text and structural DOM mutations. Connected `<style>` source changes remained an intentionally conservative exception: rebuilding `StyleSet` also forced a complete Layout Tree rebuild, even when the new cascade changed only paint or ordinary geometry on nodes that already had retained layout identity.

That fallback is broader than necessary. The final `StyleSet` can be compared against the styles stored on retained layout nodes, while structural membership changes such as `display:none` still require a fail-closed boundary.

## Decision

When connected stylesheet sources change, rebuild the `StyleSet` but revalidate retained layout before deciding whether a full layout rebuild is required.

The engine will:

- treat character-data mutations inside `<style>` as stylesheet-source changes rather than text-layout mutations;
- refresh connected structural roots first when stylesheet insertion/removal/reparenting changes both DOM structure and CSS sources;
- globally enqueue DOM nodes represented by the retained Layout Tree for style comparison against the new `StyleSet`;
- ignore style-source nodes themselves as layout candidates because `<style>` content is not represented in the rendered Layout Tree;
- route paint-only style differences through retained fragment/display-list patching;
- route supported geometry differences through retained flow relayout;
- keep formatting-context boundary changes (`display`, inline/block role, BFC establishment) on deterministic full-rebuild fallback;
- scan connected elements absent from retained layout outside structural refresh roots and force a full rebuild if the new stylesheet changes their `display:none` membership.

## Consequences

Ordinary stylesheet text edits can now report `styles_rebuilt = true` while still using `PaintOnlyReuse` or `FlowRelayout`. Existing `LayoutNodeId` identity is preserved where the formatting structure remains compatible, and retained display-list work remains available.

The visibility-membership scan deliberately favors correctness over minimal work. It closes the hidden-to-visible case that cannot be discovered by walking retained layout nodes alone. More selective rule-to-element invalidation may replace the global retained-node revalidation in later milestones without changing this correctness boundary.
