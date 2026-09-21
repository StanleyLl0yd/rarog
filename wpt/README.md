# WPT compatibility evidence

Rarog began focused Web Platform Test work in R1 and turns that foundation into explicit compatibility measurement in R6.

Upstream `web-platform-tests/wpt` remains the source of truth. Current upstream tooling supports machine-readable JSON reports through `wpt run --log-wptreport=report.json`; Rarog's R6 dashboard consumes that report shape rather than inventing a parallel test-result format.

## R1 focused preflight

`r1-focus.txt` is the historical/versioned initial scope. Entries are upstream WPT directory scopes, not claims that every test in those directories is supported or measured.

The initial R1 focus covers HTML parsing plus CSS cascade/selectors/display/box/text behavior. `crates/rarog-engine/tests/wpt_preflight.rs` keeps that focus manifest non-empty and directory-scoped.

## R6 dashboard foundation

`scripts/wpt_dashboard.py` normalizes one or more WPT `wptreport` JSON files into:

- deterministic canonical JSON;
- deterministic human-readable Markdown;
- exact Rarog and WPT commit identities;
- content-addressed SHA-256 identities for source reports;
- measured test/subtest status counts;
- expectation-known and unexpected counts.

It deliberately does **not** infer a compatibility result for unmeasured tests, directories or focus-manifest entries.

Example for a real report:

```text
python3 scripts/wpt_dashboard.py \
  --report /path/to/wptreport.json \
  --rarog-commit <40-hex-rarog-commit> \
  --wpt-commit <40-hex-wpt-commit> \
  --platform windows-11-x86_64 \
  --json-out /tmp/rarog-wpt-dashboard.json \
  --markdown-out /tmp/rarog-wpt-dashboard.md
```

Multiple non-overlapping report shards may be supplied by repeating `--report`. Duplicate test IDs across shards are rejected rather than double-counted. Ambiguous JSON with duplicate object keys and non-finite numbers is rejected.

## R6 pinned selection

`r6-selection.json` pins the first exact R6 candidate set to:

`web-platform-tests/wpt@a83afd4402cffdc876508fe9a47f916d4136099f`

It contains five concrete test files:

- three HTML parsing `testharness` tests;
- two CSS selector reftests;
- two exact `.html` reference files for those reftests.

The first denominator is intentionally HTML-mode only. XML/XHTML parser-mode qualification is not mixed into this initial render-based compatibility slice.

Every selected test/reference carries its upstream Git blob object ID. The list is a denominator for the first real execution attempt; it is **not** a pass list and contains no expected Rarog outcome.

Validate the manifest against an exact local upstream checkout with:

```text
python3 scripts/wpt_selection.py \
  --manifest wpt/r6-selection.json \
  --wpt-checkout /path/to/wpt
```

The verifier requires the checkout HEAD to match the pinned commit, verifies every declared blob against the pinned commit tree, independently hashes the selected working-tree files, validates testharness/reftest metadata and confirms the declared reference files. Manifest drift and local content drift therefore fail independently.

## Synthetic fixture

`fixtures/synthetic-wptreport.json` exists only to regression-test the normalizer. Use:

```text
python3 scripts/wpt_dashboard.py \
  --report wpt/fixtures/synthetic-wptreport.json \
  --rarog-commit bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \
  --wpt-commit cccccccccccccccccccccccccccccccccccccccc \
  --platform synthetic-linux \
  --synthetic \
  --json-out /tmp/rarog-wpt-synthetic.json \
  --markdown-out /tmp/rarog-wpt-synthetic.md
```

Synthetic output contains an explicit machine-readable marker and a prominent Markdown warning. It is not compatibility evidence. The committed fixture digest is recognized by the normalizer and is rejected unless `--synthetic` is supplied.

Tooling regression tests:

```text
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

## Evidence policy

- Record the exact upstream WPT commit for every published result.
- Record the exact Rarog commit for every published result.
- Prefer exact test selections; do not report an entire directory as passing from a partial run.
- Known failures/timeouts/crashes remain explicit.
- Missing expectation metadata remains unknown; it is not converted into a pass claim.
- Keep upstream expectations separate from Rarog implementation code.
- A synthetic fixture or dashboard tooling test never counts as measured compatibility.
- Compatibility percentages may be introduced only when the denominator is explicitly enumerated and fully measured.
