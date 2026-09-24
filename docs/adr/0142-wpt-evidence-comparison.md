# ADR-0142: WPT evidence comparison and update policy

## Status

Accepted for the R6.4 compatibility-measurement slice.

## Context

R6.1–R6.3 established a deterministic WPT dashboard, an exact file-level denominator and the first real non-synthetic evidence set. Future runs need a comparison contract that does not turn changes in upstream WPT, the selected denominator, platform or expectation metadata into apparent Rarog regressions or improvements.

A raw status ordering such as `PASS > FAIL > ERROR` is not a valid compatibility model. In particular, an `ERROR -> FAIL` transition can still leave both outcomes unexpected and does not by itself prove an improvement.

## Decision

Rarog compares only independently validated evidence bundles. Each bundle contains:

- the exact selection manifest;
- the exact raw WPT report;
- the exact evidence envelope reproduced from selection + report;
- the exact non-synthetic dashboard reproduced from the raw report.

The canonical evidence-set identity is:

- exact Rarog commit;
- exact upstream WPT commit;
- exact selection digest;
- exact raw-report digest;
- exact measured platform label.

The comparison tool is `scripts/wpt_compare.py`. It first regenerates and requires exact equality for both evidence envelopes and both dashboards. Invalid, synthetic or internally inconsistent evidence fails closed before any comparison is emitted.

### Comparison classes

Every comparison is classified as one of:

1. `same-upstream-same-denominator`;
2. `same-upstream-changed-denominator`;
3. `changed-upstream-same-logical-selection`;
4. `changed-upstream-changed-denominator`.

The logical selection signature uses the selected test path, execution kind and reference relation/path. Upstream blob IDs are excluded from the logical signature so the same logical test set can be recognized across upstream revisions; blob drift is reported separately.

### Direct behavior interpretation

A test result transition can be called a **regression** or **improvement** only when:

- the comparison is class 1;
- the exact selection digest is unchanged;
- the platform is unchanged;
- the selected test/reference source identity is unchanged for that test;
- the expectation metadata for that test is unchanged.

Within that boundary:

- expected -> unexpected = `regression`;
- unexpected -> expected = `improvement`;
- unexpected -> unexpected with a different observed status = `changed-unexpected-status`;
- expected -> expected with a different allowed observed status = `changed-expected-status`.

The last two are deliberately not ranked.

If the upstream revision, denominator, platform, source identity or relevant expectation basis changed, status differences are reported as `observed-change-not-directly-comparable` rather than as behavior verdicts.

### Scope and upstream changes

Added and removed tests are always listed explicitly. A changed logical definition for an existing path is also explicit.

Selected test/reference blob changes are reported independently of result changes. This matters especially when the upstream WPT revision changes: a status difference alongside changed source is not attributed to Rarog without separate evidence.

Expectation changes are likewise reported independently. A new expectation can change whether an observation is “unexpected” without changing Rarog behavior.

### Output

The comparator emits deterministic machine-readable JSON and human-readable Markdown with:

- evidence identities;
- comparison class and direct-comparability status;
- added/removed/changed logical scope;
- selected source-content drift;
- expectation drift;
- exact per-test before/after observed state;
- bounded transition counts.

It emits no compatibility percentage, aggregate score or ranking.

## Consequences

- Same-denominator, same-platform status regressions can be identified without hiding failures.
- Denominator growth/shrinkage cannot masquerade as percentage movement.
- Upstream WPT changes remain distinguishable from Rarog changes.
- Unsupported `ERROR`, `TIMEOUT`, `CRASH` and other statuses remain explicit rather than being collapsed into one failure bucket.
- The first R6.3 baseline can be used as an immutable comparison anchor for later runs.
- Updating the selected denominator or upstream revision remains allowed, but such comparisons are labeled structurally different instead of directly behavior-comparable.
