# ADR-0092: Preserve content-distribution computed semantics

## Status

Accepted.

## Context

ADR-0091 adds the Grid final auto-track stretch phase, whose activation depends on the Grid container's content-distribution property being `normal` or `stretch`.

Rarog's bounded alignment model previously collapsed these values too early:

- `justify-content: normal` parsed directly to `FlexStart`;
- `justify-content: stretch` was rejected;
- `align-content: normal` parsed directly to `Stretch`;
- the initial computed values therefore stored used Flex behavior instead of the specified Box Alignment keywords.

That was sufficient for the earlier Flex-only slices, but it loses information Grid needs. CSS Box Alignment defines `normal` as behaving as `stretch` for both Flex and Grid containers. For Flex main-axis `justify-content`, `stretch` in turn behaves as `flex-start` because main-axis stretching is controlled by flexing.

## Decision

Rarog preserves the bounded specified content-distribution keywords in computed style.

`JustifyContent` gains:

- `Normal`;
- `Stretch`.

`AlignContent` gains:

- `Normal`.

The initial computed value of both properties becomes `Normal`.

The parser preserves `normal` and `stretch` distinctly instead of collapsing them.

Flex layout translates these computed values only at the used-value boundary:

- `justify-content: normal` and `stretch` both use the existing main-axis start alignment;
- `align-content: normal` and `stretch` both use the existing cross-axis stretch behavior.

Thus existing Flex rendering remains unchanged while computed style retains the information needed by Grid.

Grid formatting-root invalidation now treats changes to `justify-content` and `align-content` as layout-affecting.

The preserved `Normal` / `Stretch` values are the only bounded content-distribution values that authorize the later final auto-track stretch phase. Positional and space-distribution values must not be inferred as stretch merely because older Flex used-value code mapped defaults to start/stretch behavior.

No Grid track geometry changes in this slice.

## Consequences

The computed-style model remains Copy and bounded.

Flex behavior keeps the existing output while becoming more faithful to Box Alignment's computed/used-value separation.

The next Grid integration slice can enable final auto-track stretch precisely for `normal` and `stretch`, rather than inferring it from a Flex-specific collapsed value.

## Deferred

This ADR does not add:

- baseline content alignment;
- safe/unsafe overflow alignment;
- left/right content positioning;
- `place-content`;
- full Grid content-position distribution after track sizing;
- block-container content alignment.

Those remain separate bounded standards work.
