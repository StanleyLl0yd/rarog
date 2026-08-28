# ADR-0026: Style sharing and relational invalidation

## Status

Accepted.

## Context

The bootstrap style system originally invalidated only the element whose `id`, `class` or inline style changed. That is sufficient while selectors are local simple compounds, but descendant and sibling combinators make selector matching depend on relationships outside the subject element. A future style cache also needs an explicit statement of which inputs make two computed styles shareable.

## Decision

Style rules may carry `SelectorDependency` metadata independently of their current bootstrap matching implementation. Each dependency identifies a trigger `SelectorInvalidationKey` and a conservative scope: `Descendants` or `FollowingSiblings`. `StyleSet` aggregates these dependencies and the engine feeds them into DOM-mutation invalidation.

For `id` and `class` mutations, invalidation is based on the changed attribute category rather than only the new value. This intentionally over-invalidates because the R0 mutation record does not retain the old attribute value; it therefore remains correct when a selector trigger is removed. Structural insertion and reparenting conservatively mark affected subtrees whenever relational dependency scopes are present.

R0 defines `StyleSharingKey` from the inputs sufficient for its local selector model: namespace, local tag name, ID, canonicalized class set and inline style. `StyleSet::local_style_sharing_safe` returns false when relational dependency metadata is present. No process-global computed-style cache is introduced in R0. A future cache must be bounded to a document/style-set lifetime and include or account for every observable contextual input before sharing.

## Consequences

- The DOM mutation journal remains independent of CSS selector implementation details.
- A standards-oriented selector parser can populate relational dependency metadata without changing engine invalidation ownership.
- Trigger removal is conservatively correct despite the current mutation journal storing no old attribute value.
- Sibling/descendant invalidation may over-invalidate in R0; correctness is preferred over premature precision.
- Current simple selectors remain local and produce no relational dependencies automatically.
- This ADR does not implement combinator parsing/matching, inheritance, pseudo-class state or a production computed-style cache.
