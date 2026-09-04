# ADR-0070: Definite-row auto-height flex stretch

## Status

Accepted.

## Context

ADR-0068 introduced container cross-axis alignment and ADR-0069 added per-item `align-self`, but the bounded flex-item path still required an explicit item height. That meant the initial Flexbox behavior of stretching an auto-height item could be represented but not actually sized.

Implementing arbitrary auto cross-size items requires intrinsic cross-size measurement and later baseline work. A narrower standards-oriented slice is possible when the single-line flex container already has a definite used cross size and the item's effective alignment is `stretch`.

## Decision

Rarog accepts an auto-height flex item in the current horizontal single-row path only when:

- the flex row has a definite used content cross size;
- the item's effective cross alignment is `stretch`, after resolving `align-self:auto` through the container's `align-items`.

Otherwise the existing fail-closed behavior remains.

The stretched content height is derived from the definite line cross size by subtracting the item's vertical margins, borders and padding, flooring the result at zero, then applying the existing max-then-min content-height clamp.

The resolved stretched content height is carried separately from computed style. `height:auto` remains `auto` in the fragment's computed style; only used geometry receives the override.

Private box/flex fragment builders accept an optional used content-height override. For a stretched flex item that is itself a flex container, that used height is also propagated as a definite cross-size signal to its nested flex formatting context.

An auto-height item whose effective alignment is not stretch, or whose row lacks a definite cross size, continues to reject the current flex-row slice rather than silently using block-flow or viewport-derived geometry.

## Consequences

The common definite-height single-row Flexbox case now performs real auto-height stretch while preserving the existing explicit-height behavior.

Nested flex items can retain definite cross-size information created by stretch without rewriting their computed style.

The slice does not claim general intrinsic auto cross-size support.

## Deferred

Later R3 slices still own:

- intrinsic auto-height flex items for non-stretch alignment;
- baseline alignment;
- auto margins;
- wrapping and multi-line alignment;
- reverse directions and writing modes;
- intrinsic/auto flex bases and full main-size freeze/redistribution.
