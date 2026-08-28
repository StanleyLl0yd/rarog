# ADR-0031: R0 Ember exit boundary

## Status

Accepted.

## Context

R0 exists to prove Rarog's architecture with a deterministic end-to-end rendering path, explicit ownership boundaries and a first stateful invalidation/reuse experiment. During R0, the backlog also accumulated future implementation items such as standards HTML/CSS parsers and concrete Windows platform services. Treating those future items as Ember blockers would make the milestone boundary inconsistent with the roadmap and would turn R0 into an open-ended standards/platform implementation phase.

## Decision

R0 is complete when its architectural contracts and deterministic correctness gates are present and green. Standards breadth and concrete platform service implementations are assigned to the roadmap milestone that first needs them.

The completed R0 contract includes:

- deterministic DOM/style/layout/fragment/display-list/framebuffer identity;
- checked DOM mutation ownership and engine-owned mutation-history consumption;
- replaceable HTML input/diagnostic and style/cascade/invalidation boundaries;
- derived layout/fragment identities plus bounded incremental reuse/fallback paths;
- text segmentation/shaping backend contracts without requiring a production shaper;
- structural display-list scopes, retained-range validation and damage-aware raster foundations;
- render observability without performance claims;
- Engine/View embedder ownership, host policy, callbacks and resource budgets;
- a platform-neutral host contract and a Windows-specific host seam;
- Windows-primary, Linux-portability and MSRV CI.

The standards-oriented HTML and CSS parsers move to R1. Concrete Windows font/text, input/IME, GPU/compositor, sandbox/process and accessibility services remain in their roadmap milestones. Native reference-browser window/UI work remains later browser work.

A dedicated `r0_exit` integration test verifies that the historical R0 backlog has no unchecked checklist items and re-checks deterministic end-to-end rendering. The normal CI correctness and incremental gates remain authoritative.

## Consequences

- R0 has a finite, auditable completion point instead of expanding with every future subsystem.
- Completing R0 does not imply standards compliance, Web compatibility, production security or competitive performance.
- New feature breadth belongs to R1+ roadmap sections unless an actual R0 invariant is discovered to be incorrect.
- Workspace version `0.1.0` can identify the Ember source milestone once the exit merge commit passes post-merge CI.
