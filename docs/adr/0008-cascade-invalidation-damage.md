# ADR-0008: Explicit cascade, invalidation and paint damage boundaries

- Status: Accepted
- Date: 2026-08-26

## Context

Rarog R0 already separates mutable DOM state from derived layout nodes, fragments and display-list commands. The next architectural risk is allowing style matching, DOM mutations and paint updates to become implicit cross-subsystem side effects.

A real browser engine eventually needs incremental style, layout and paint. If those systems are coupled through object pointers or hidden callbacks, later parallelism, process isolation, crash recovery and deterministic testing become much harder.

## Decision

Rarog will model the path from mutation to pixels as explicit data transformations.

### Style and cascade

The style subsystem owns explicit representations for:

- stylesheet sources;
- cascade origin and layer identity;
- selectors and specificity;
- typed property IDs/values;
- deterministic source order.

The R0 selector/parser surface is deliberately small. It establishes the data model without claiming CSS standards completeness.

### Mutation and invalidation

The DOM records accepted semantic mutations with monotonically increasing document generations. The DOM crate does not depend on CSS, layout or paint types.

Style/layout code consumes mutation records and produces conservative dirty flags. Selector invalidation keys expose which simple tag/ID/class keys matter to the current bootstrap selector set.

R0 may rebuild complete derived trees after invalidation. Persistent dirty-state application and an incremental dependency graph are later work.

### Paint identity and damage

Paint commands receive deterministic display-item IDs. Damage is computed by comparing previous/current display lists by ID and command value rather than by allowing layout to draw directly or mutate a framebuffer.

The first damage region is intentionally conservative and rectangle-based. Retained display lists, clipping, transforms, occlusion and compositor-specific damage are later layers.

### Determinism

R0 exposes deterministic snapshots of the major derived representations and stable non-cryptographic hashes for the software framebuffer and combined render signature. CI treats these as correctness/regression tools, not security primitives.

## Consequences

Positive:

- DOM remains independent of style/layout/paint implementation details.
- Future incremental scheduling has explicit mutation and dirty inputs.
- Cascade decisions can be tested independently of layout.
- Display-list changes can be measured without rasterizing the whole architecture into paint side effects.
- Deterministic regression failures can identify which pipeline representation changed.

Costs:

- R0 carries extra IDs, mutation records and snapshot code before an incremental renderer exists.
- Current invalidation is conservative and may mark more work than necessary.
- Current display-item IDs are stable only under the deterministic R0 tree/fragment construction rules and will need a stronger retained identity strategy later.

## Non-goals

This ADR does not claim:

- CSS Cascade/Selectors compliance;
- incremental style or layout execution;
- a retained compositor;
- optimal damage coalescing;
- cryptographic integrity from the regression hashes.
