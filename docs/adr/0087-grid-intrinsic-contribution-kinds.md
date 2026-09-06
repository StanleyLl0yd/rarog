# ADR-0087: Distinct Grid intrinsic contribution kinds

## Status

Accepted.

## Context

ADR-0086 established an order-independent spanning distribution primitive that accepts an already-selected size contribution. It intentionally does not decide which CSS Grid contribution class a sizing phase should use.

The Grid track-sizing algorithm distinguishes multiple item contributions, including the minimum contribution, min-content contribution and max-content contribution. These values are related but are not interchangeable. The specification also defines the invariant `minimum contribution <= min-content contribution <= max-content contribution`.

Rarog's earlier bounded single-span `GridItemContribution` stores one inline and one block measurement for the existing simplified auto-track path. Reinterpreting that public type as all intrinsic contribution classes would blur the algorithm boundary and make later spanning phases depend on implicit assumptions.

## Decision

Rarog introduces crate-internal intrinsic contribution contracts:

- `GridIntrinsicContributionKind::{Minimum, MinContent, MaxContent}`;
- `GridAxisIntrinsicContributions`, storing the three contribution values for one axis;
- `GridIntrinsicContributions`, storing independent inline and block contribution sets for one layout node.

Each axis contribution set must be finite, non-negative and ordered as `minimum <= min-content <= max-content`. This validates a specification invariant but does not make the values interchangeable: callers must still select the semantic contribution required by the relevant CSS sizing phase.

The existing public `GridItemContribution` remains unchanged for source compatibility and continues to serve the previously implemented bounded single-span path.

No CSS-visible sizing behavior is changed by this ADR.

## Consequences

Later intrinsic sizing phases can name the contribution class they consume instead of passing an ambiguous scalar.

Inline and block contribution semantics can evolve independently.

The existing public Grid geometry and contribution APIs remain source-compatible.

## Deferred

This ADR does not define how Rarog computes:

- the CSS Grid automatic minimum contribution;
- limited min-content or limited max-content contributions;
- transferred size suggestions or aspect-ratio effects;
- block-axis min-content/max-content contributions under every writing mode;
- contribution selection for each spanning phase.

Those computations must be added in bounded slices before the spanning primitive is connected to CSS-visible layout.
