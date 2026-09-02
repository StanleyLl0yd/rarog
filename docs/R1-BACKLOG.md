# R1 — Flame backlog

Status: **in progress**.

Tracking issue: #39.

## A — Standards HTML parsing

- [x] Land the `html5ever` adapter behind Rarog-owned parser types.
- [x] Add focused adapter tests for implied structure, character references, tree-builder insertion rules and foreign namespaces.
- [x] Add differential bootstrap-versus-standards fixtures to identify intentional semantic changes.
- [x] Switch the engine default parser to the standards adapter after resource/no-panic gates cover the new path and document scaffolding has correct layout semantics.
- [x] Retire the bootstrap parser after the migration gate is green.
- [x] Expand the focused R1 WPT manifest with the first executable HTML parser subset.

## B — Standards CSS and cascade

- [x] Add a standards-oriented CSS tokenizer/parser adapter.
- [x] Add combinators, attribute selectors and pseudo-classes in measured slices.
- [x] Add `!important`, inheritance and CSS-wide values.

## C — Formatting contexts and resources

- [x] Add block formatting context foundations.
  - [x] Collapse adjoining vertical margins between in-flow block siblings, including negative margins.
  - [x] Add parent/child and empty-block margin collapsing boundaries.
  - [x] Add block auto/min/max sizing and explicit BFC boundary rules.
- [ ] Add inline formatting context foundations.
  - [x] Add explicit atomic inline boxes with horizontal line packing and block interruption.
  - [x] Share one line builder across text and inline boxes while preserving Unicode shaping and line-break source ranges.
  - [ ] Add baseline/vertical-align behavior and inline fragmentation.
- [ ] Add image resource abstraction.

## D — Production text path

- [ ] Connect a production OpenType shaping backend behind the existing shaping request boundary.
- [ ] Add the first Windows font discovery/text adapter.

## E — Incremental rendering breadth

- [ ] Extend invalidation into standards-aware style/layout dependencies.
- [ ] Extend retained/damage-aware paint across richer formatting and stacking behavior.

## Scope boundary

JS/WebIDL, Fetch/networking, GPU/compositor, process sandboxing and browser UI remain on later roadmap milestones.
