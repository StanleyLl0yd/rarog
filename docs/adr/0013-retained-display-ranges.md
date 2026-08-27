# ADR-0013: Retained display ranges

## Status

Accepted for R0.

## Context

The first retained display-list experiment identified affected commands by an unordered set of display-item IDs. That is sufficient for simple paint commands, but it can silently become unsafe once clip and stacking scopes are explicit because removing separated matching commands can splice across unrelated structural boundaries.

## Decision

Retained display-list updates identify the previous subtree output as one exact contiguous command range. The replacement is accepted only when the old command-ID sequence appears contiguously in the retained list and both the replacement and final candidate preserve display-item uniqueness and balanced structural scopes.

The update is atomic: changes are applied to a candidate list first and committed only after all invariants pass. A failed retained patch leaves the original display list unchanged so the engine can fall back to a full display-list rebuild.

## Consequences

Retained updates no longer infer ranges from unordered ID membership and cannot partially mutate the retained list on validation failure. This is still an R0 structural foundation; true clip-, stacking-, and fragmentation-aware damage calculation remains a later paint milestone.
