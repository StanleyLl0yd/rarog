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
- [ ] Add `!important`, inheritance and CSS-wide values.

## C — Formatting contexts and resources

- [ ] Add block formatting context foundations.
- [ ] Add inline formatting context foundations.
- [ ] Add image resource abstraction.

## D — Production text path

- [ ] Connect a production OpenType shaping backend behind the existing shaping request boundary.
- [ ] Add the first Windows font discovery/text adapter.

## E — Incremental rendering breadth

- [ ] Extend invalidation into standards-aware style/layout dependencies.
- [ ] Extend retained/damage-aware paint across richer formatting and stacking behavior.

## Scope boundary

JS/WebIDL, Fetch/networking, GPU/compositor, process sandboxing and browser UI remain on later roadmap milestones.
