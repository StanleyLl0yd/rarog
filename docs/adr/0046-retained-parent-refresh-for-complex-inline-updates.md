# ADR-0046 — Retained parent refresh for complex inline updates

Status: Accepted

## Context

R1 retained rendering already handles ordinary text reflow, block geometry, structural DOM mutations, stylesheet-source changes and formatting-boundary transitions without rebuilding the complete Layout Tree. Several inline-specific cases remained conservative full-rebuild boundaries:

- geometry changes on inline owners;
- style changes on DOM nodes represented by multiple inline fragments;
- geometry-changing style updates combined with `CharacterData` mutations in one update;
- nested, multi-leaf and pure inline subtrees whose style changes require rebuilding a shared line context;
- geometry-sensitive inline properties such as `vertical-align`.

These cases cannot be safely patched as one retained fragment because the changed node may participate in several fragments or share line construction with siblings. They do, however, have the same safe structural boundary introduced for formatting-membership changes: the nearest retained parent whose layout subtree can be rebuilt while preserving unaffected outer identity.

## Decision

Route complex inline and mixed text/geometry style updates through retained parent refresh.

When a style candidate maps to multiple fragments, or a layout-affecting style change occurs while text is dirty or either the old/new style participates in inline layout, the engine finds the nearest retained structural parent and schedules it in the existing formatting-relayout root set rather than forcing `FullRebuild`.

The roots are minimized, rebuilt through `refresh_layout_subtrees`, and then passed to `relayout_fragment_flow`. Covered text/style work is removed because the refreshed subtree has already consumed it. The retained display-list flow suffix is then patched through the existing paint path.

If no retained parent can cover the update, the fail-closed full rebuild remains.

## Consequences

Inline geometry, fragmented/nested inline styling, multi-leaf inline ownership, vertical alignment and mixed `CharacterData` plus geometry-style updates now use `FlowRelayout` when a retained parent exists. Regression gates compare framebuffer output—and where applicable fragment/display-list snapshots—against a fresh render.

This removes the last known normal-content inline-specific full-layout fallbacks. Remaining full rebuild paths are reserved for lost mutation history, missing/unsupported retained structural coverage, safety/resource limits, or fail-closed recovery when retained refresh/patch invariants cannot be satisfied.
