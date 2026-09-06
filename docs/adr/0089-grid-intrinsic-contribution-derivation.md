# ADR-0089: Bounded Grid intrinsic contribution derivation

## Status

Accepted.

## Context

ADR-0087 introduced distinct Grid minimum, min-content and max-content contribution contracts, while the existing Grid fragment builder still collapsed each item to one legacy scalar before track sizing.

That legacy scalar is sufficient to preserve the bounded max-content behavior introduced in #182, but it discards information that the CSS Grid intrinsic track-sizing phases need before they can be implemented correctly.

Rarog's current retained style/layout subset is narrower than the full automatic minimum-size rules. It has definite/auto width and min/max dimensions, intrinsic min-content/max-content measurements, margins, padding and borders, but does not yet represent scrollable overflow, aspect-ratio transferred size suggestions, replaced-element sizing or flexible Grid tracks.

## Decision

The Grid fragment builder now derives layout-owned semantic contribution sets directly.

For the inline axis:

- min-content contribution is the retained intrinsic min-content outer size plus horizontal margins;
- max-content contribution is the retained intrinsic max-content outer size plus horizontal margins;
- minimum contribution equals the min-content contribution within the currently supported subset.

The equality between minimum and min-content is deliberately bounded. It is not a general CSS rule; it is valid only while the currently unsupported automatic-minimum conditions that could select or clamp other size suggestions are absent from Rarog's retained model.

For the block axis, the existing post-column natural border-height measurement is retained and represented as a degenerate semantic set where minimum, min-content and max-content are equal. Rarog does not claim full block-axis intrinsic sizing from that representation.

A crate-internal semantic track resolver accepts the contribution set plus an explicit contribution kind. The public legacy `GridItemContribution` API remains source-compatible and adapts its scalar measurements into semantic sets.

For this slice, production Grid layout still selects `MaxContent` from the semantic set. Therefore the CSS-visible max-content track geometry introduced by #182 is intentionally unchanged.

## Consequences

The fragment builder no longer destroys inline min-content information before Grid track sizing.

Future base-size and growth-limit phases can select semantic contribution kinds explicitly without coupling layout to CSS parser AST types.

Existing public Grid contribution APIs and current rendered geometry remain compatible.

## Deferred

This ADR does not implement:

- the complete CSS Grid automatic minimum-size conditions;
- scrollable-overflow effects on automatic minimums;
- transferred size suggestions from preferred aspect ratios;
- replaced-element contribution rules;
- percentage-dependent preferred sizes;
- independent block-axis min-content/max-content measurement;
- switching auto-track base sizes from the #182 max-content compatibility behavior to minimum contributions;
- max-content growth-limit resolution, Maximize Tracks, flexible tracks or Stretch auto Tracks;
- CSS-visible intrinsic sizing for items spanning multiple tracks.

Those phases require additional bounded contracts before the existing compatibility selection can be removed.
