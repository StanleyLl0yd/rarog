# ADR-0088: Span-ordered Grid intrinsic sizing rounds

## Status

Accepted.

## Context

ADR-0086 introduced order-independent planned base-size increases for a single distribution round. CSS Grid intrinsic track sizing does not process every spanning item in one global round: after non-spanning work, spanning items are considered in increasing order of span.

This ordering matters because larger-span items must account for base-size increases already committed by smaller-span rounds. Merely sorting an input list while deferring every mutation until the end would not reproduce that state transition.

Rarog still does not have enough contribution derivation to connect these rounds to CSS-visible spanning layout. A separate synthetic phase runner allows the ordering contract to be tested before that wiring.

## Decision

Rarog adds a layout-internal span-ordered base-size runner.

For a supplied set of synthetic spanning size contributions, the runner:

1. collects the distinct positive spans represented by the inputs;
2. processes those spans in increasing numeric order;
3. forms one contribution round for each span;
4. plans that round's increases against the current track sizing state using ADR-0086's order-independent planner;
5. applies the round's planned increases before moving to the next larger span.

Items within the same span therefore see identical pre-round state and remain input-order independent, while larger-span rounds see the committed effects of all smaller spans.

Gap accounting is recomputed against the current base sizes in every round.

The runner remains crate-internal and synthetic. It does not select minimum/min-content/max-content contribution classes and does not remove the CSS-facing fail-closed rule for intrinsic multi-track spans.

## Consequences

The phase-ordering part of CSS Grid intrinsic spanning sizing now has an independently testable layout-owned contract.

A regression that accidentally collapses all spans into one distribution round produces observably different base sizes and is caught by the primitive tests.

Current CSS-visible Grid behavior remains unchanged.

## Deferred

Before span-ordered rounds can be used by CSS Grid layout, later slices still need:

- derivation of the semantic contribution class required by each intrinsic sizing phase;
- the automatic minimum contribution rules;
- min-content and max-content growth-limit phases;
- richer affected-track and beyond-limit distribution rules for additional track sizing functions;
- flexible track handling;
- CSS-facing spanning enablement only after those contracts compose correctly.
