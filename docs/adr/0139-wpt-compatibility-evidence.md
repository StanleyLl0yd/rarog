# ADR-0139: Content-addressed WPT compatibility evidence

## Status

Accepted for R6 compatibility measurement.

## Context

Rarog already carries a small R1 WPT focus manifest and focused upstream parser fixtures, but R6 is the first milestone whose purpose is compatibility qualification itself.

A compatibility dashboard is dangerous if it can silently change its denominator, omit failures, confuse missing expectations with success, or summarize results without identifying the exact engine and upstream revisions that produced them. Tooling fixtures are useful for testing the dashboard but are not compatibility evidence.

Upstream WPT provides machine-readable `wptreport` JSON output. Rarog needs a narrow evidence boundary above that report format without vendoring WPT or making dashboard tooling authoritative over engine behavior.

## Decision

R6 WPT dashboards normalize supplied `wptreport` files into a Rarog-owned evidence schema with:

- an exact lowercase 40-hex Rarog commit;
- an exact lowercase 40-hex upstream WPT commit;
- an explicit platform label;
- a canonical SHA-256 digest for each input report after deterministic JSON serialization;
- exact test IDs and test/subtest observed statuses;
- expectation metadata when present;
- unexpected classification only when expectation metadata exists and the observed status is outside it;
- deterministic sorting and deterministic JSON/Markdown generation.

Duplicate JSON object keys, non-finite JSON numbers, duplicate test IDs across report shards and duplicate subtest names inside one test are rejected. Empty result sets are rejected.

Missing expectation metadata is preserved as unknown and does not increment unexpected counts. The dashboard reports only records actually present in its supplied reports. It does not manufacture a directory denominator, compatibility percentage or result for unmeasured tests.

Synthetic fixture dashboards carry an explicit machine-readable `synthetic: true` marker and a prominent human-readable non-evidence warning. The committed fixture's canonical digest is denylisted from non-synthetic output so omitting the CLI flag cannot accidentally turn that fixture into publishable evidence.

The normalizer is stdlib-only tooling under `scripts/`; it adds no product dependency or runtime authority. Its regression suite runs in the protected Linux/Verify path and in Repository Full Audit.

## Consequences

- Dashboard output is reproducible for identical reports and metadata.
- Report file paths do not affect evidence identity; canonical content digests do.
- Multiple non-overlapping WPT shards can be combined safely.
- A future signed compatibility profile can reference canonical dashboard inputs rather than mutable local paths.
- First real WPT execution remains separate work; passing the synthetic fixture cannot be reported as Rarog WPT compatibility.
- If upstream WPT report semantics change, Rarog must deliberately update this adapter and its fixtures rather than silently reinterpret old evidence.
