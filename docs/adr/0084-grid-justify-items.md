# ADR-0084: Bounded Grid justify-items default alignment

## Status

Accepted.

## Context

ADR-0079 introduced bounded Grid self-alignment through `justify-self` and `align-self`. Its inline-axis default was temporarily hard-coded so `justify-self:auto` behaved as stretch because `justify-items` was not yet represented.

ADR-0083 subsequently added intrinsic automatic sizing for non-stretch Grid items, so a container-level inline-axis default can now select start/end/center without forcing an explicit item width.

CSS Box Alignment defines `justify-items` as the parent-side default referenced by a child's `justify-self:auto`. The complete property also carries legacy HTML alignment behavior, baseline alignment and overflow-position syntax that are outside the current bounded R3 alignment model.

## Decision

Rarog adds a Copy `JustifyItems` computed-style value with the bounded variants:

- `normal`;
- `stretch`;
- `start` / `flex-start`;
- `end` / `flex-end`;
- `center`.

The initial bounded value is `normal`. For the currently supported non-replaced Grid boxes, `normal` resolves to the existing stretch/start result.

`justify-self:auto` now resolves through the parent Grid container's `justify-items`. Explicit `justify-self` continues to override the container default.

A non-stretch automatic inline size uses the intrinsic sizing path established by ADR-0083. Stretch sizing, explicit sizes, physical margins and min/max constraints remain unchanged.

Changes to a Grid container's `justify-items` invalidate the Grid formatting root for retained layout, matching the existing `align-items` behavior.

The parser deliberately rejects `legacy`, baseline alignment, safe/unsafe overflow alignment, left/right and other unsupported Box Alignment forms rather than approximating them.

## Consequences

Grid containers can now set a bounded default inline alignment without repeating `justify-self` on every item.

The computed-style model remains Copy and no parser AST type enters layout.

The old hard-coded `justify-self:auto => stretch` rule is replaced by an explicit parent default-alignment contract while preserving the same default visual result for currently supported Grid boxes.

## Deferred

Later alignment work still owns:

- legacy HTML alignment semantics;
- baseline self/default alignment;
- auto-margin precedence;
- safe/unsafe overflow alignment;
- left/right and writing-mode-sensitive expansion;
- `place-items` / `place-self` shorthands;
- replaced-element-specific `normal` behavior.
