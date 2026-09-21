# ADR-0140: Pinned file-level WPT selection

## Status

Accepted for R6 compatibility qualification.

## Context

R6.1 established deterministic normalization of WPT result reports, but a trustworthy dashboard also needs an explicit denominator. The historical R1 focus list contains directory scopes and intentionally never meant that every test in those directories was selected, executable or passing.

R6 must not choose or remove tests dynamically according to current Rarog behavior. A first real run therefore needs a small exact upstream source revision and a file-level selection whose contents can be independently checked before any execution result is interpreted.

## Decision

Rarog stores the initial R6 selection in `wpt/r6-selection.json`.

The selection:

- identifies `web-platform-tests/wpt` and one exact 40-hex upstream commit;
- contains a strictly sorted, unique list of concrete test files;
- classifies each test by standards area and execution kind;
- records the exact upstream Git blob object ID for every selected test;
- records normalized `match`/`mismatch` reference paths plus exact Git blob IDs for reftests;
- does not contain expected Rarog pass/fail outcomes.

The initial selection is deliberately small: three HTML parsing testharness files and two CSS selector reftests whose tests and references are all `.html`. Keeping the first denominator in HTML parsing mode avoids conflating future XML/XHTML parser qualification with selector/render results. Small scope makes the first execution boundary inspectable; it is not a claim that these are the only important tests or that any selected test currently passes.

`scripts/wpt_selection.py` validates the manifest and can verify it against a local upstream checkout. Verification requires the checkout HEAD to equal the pinned commit. It verifies every declared blob against the pinned commit tree, independently hashes selected working-tree files with Git, confirms testharness files actually load `/resources/testharness.js`, parses reftest reference metadata, resolves references to normalized repository-relative paths, and verifies reference blobs against both the commit tree and working tree.

Malformed JSON, unknown fields, non-normalized/traversing paths, unsorted or duplicate tests/references, unsupported kinds/relations, wrong checkout commits, missing files and content drift fail closed.

The verifier is stdlib-only and its synthetic temporary-Git tests run through the compatibility-tooling test discovery already required by protected Linux/Verify and Repository Full Audit.

## Consequences

- The selected set can be reviewed without executing Rarog.
- Later execution cannot silently shrink the denominator to hide failures.
- Upstream source drift is visible as a commit or blob change.
- A manifest blob that does not belong to the pinned commit is rejected, and local checkout modification of a selected test/reference is detected independently even if HEAD still matches.
- The manifest itself is selection evidence, not compatibility evidence: no test has a result until a real execution emits a non-synthetic report.
- Updating the upstream revision or selected paths requires an explicit reviewed manifest change and must remain distinguishable from changes in Rarog behavior.
