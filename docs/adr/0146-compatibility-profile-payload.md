# ADR-0146: Canonical compatibility profile payload

## Status

Accepted for the R6.8 compatibility-profile slice.

## Context

R6.1–R6.7 produced independently reproducible compatibility evidence with different purposes and different measured Rarog commits:

- selected WPT evidence measures an exact five-test denominator on Rarog `dcd349dd37b0340ec67a2fb8d36b13980e2fd918`;
- the first real-Web corpus baseline measures one bounded live scenario on Rarog `2301370b6ccce06a8839a94a75dfeee12e3a3499`.

A profile that simply stamped both records with the current repository commit would destroy evidence identity. Likewise, collapsing WPT FAIL/ERROR and real-Web external/engine states into one score would discard the semantics established by the underlying evidence contracts.

## Decision

The canonical profile is `compatibility/profile.json`, schema version 1 / profile revision 1.

Every evidence entry contains:

- a stable evidence ID and evidence kind;
- the exact Rarog commit that was actually measured;
- exact upstream/corpus source identity;
- exact canonical digest and path of the raw evidence;
- exact canonical digest and path of the normalized evidence;
- measured platform/environment;
- exact measured scope identity;
- a bounded kind-specific observed summary.

The initial profile contains exactly one WPT entry and one real-Web entry.

### Reproduction before inclusion

`scripts/compat_profile.py` does not trust hand-entered digests.

For WPT it:

1. validates the committed selection;
2. rebuilds the evidence envelope from selection + raw wptreport;
3. requires exact equality with the committed evidence JSON;
4. rebuilds the non-synthetic dashboard;
5. requires exact equality with committed dashboard JSON and Markdown;
6. computes the normalized dashboard digest.

For real-Web it:

1. validates the committed corpus;
2. rebuilds the baseline from corpus + raw execution;
3. requires exact equality with committed baseline JSON and Markdown;
4. computes the normalized baseline digest.

Only after those checks does an evidence entry enter the profile.

### Kind separation

WPT observed status counts remain WPT semantics. The initial profile therefore explicitly retains two FAIL and three ERROR test results and five unexpected tests.

Real-Web evidence retains scenario outcome category/detail plus external dependency state. The initial profile therefore explicitly retains the completed observation and the available primary-document dependency.

No compatibility percentage, aggregate score, ordering or cross-kind status is defined.

### Profile identity versus measurement identity

`schema_version` and `profile_revision` identify the payload contract and profile lineage. They do not identify an engine measurement.

Each evidence entry's `measured_rarog_commit` remains authoritative for that measurement. The repository commit that happens to contain `profile.json` must not be interpreted as replacing those historical measured commits.

### Strictness

The profile validator rejects:

- unknown root/entry/kind-specific keys;
- unknown evidence kinds;
- malformed commit/digest/path identities;
- unsorted or duplicate evidence IDs;
- duplicate evidence identities;
- unsorted/duplicate scope IDs;
- malformed or internally inconsistent observed counts/states.

## Consequences

- Future signed profiles can sign a deterministic payload without rewriting historical evidence identity.
- Historical WPT and real-Web evidence can coexist without implying a common measured engine revision.
- Adding future evidence remains an explicit profile revision rather than silent score movement.
- Signing is intentionally not part of R6.8; reproducible content identity must be established first.
