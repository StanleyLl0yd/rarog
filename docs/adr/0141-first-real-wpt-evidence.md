# ADR-0141: First real selected-WPT evidence boundary

## Status

Accepted for the R6.3 compatibility-measurement slice.

## Context

R6.1 established deterministic normalization of upstream WPT `wptreport` JSON. R6.2 pinned an exact upstream revision and an explicit five-test file-level denominator, but neither slice executed that denominator with Rarog.

The current Rarog shell is not a WebDriver/BiDi browser surface. It renders HTML statically and SpiderMonkey is not yet integrated with the document/global-object behavior required by WPT testharness. Treating either limitation as permission to omit selected tests would make the measured denominator outcome-dependent.

A first real evidence set therefore needs to execute the exact R6.2 denominator through upstream wptrunner while keeping unsupported capabilities visible.

## Decision

R6.3 uses an external upstream-wptrunner Product named `rarog` and keeps the adapter outside production Web semantics.

The selected execution boundary is:

- exact upstream WPT revision `a83afd4402cffdc876508fe9a47f916d4136099f`;
- exact five-test denominator from `wpt/r6-selection.json`;
- a test-only viewport-aware Rarog renderer over the existing `render_html` path;
- wptrunner `NullBrowser` / `ConnectionlessProtocol` lifecycle;
- upstream `RefTestImplementation` as the authority for reference relation, fuzzy matching and reftest result conversion;
- PPM output from Rarog converted to PNG in the Python adapter with Pillow already required by the pinned wptrunner environment;
- explicit testharness `ERROR` while real Rarog DOM/script testharness integration is absent.

The adapter is allowlisted to selected tests and their exact reference files. It does not implement a parallel comparison algorithm, remove selected failures, add Rarog expectations or specialize standards-engine behavior for the dashboard.

The dedicated `R6 Selected WPT` workflow verifies the pinned checkout and selection before execution, invokes upstream wptrunner with exactly the selected `reftest` and `testharness` types, requires a non-empty raw `wptreport`, and then requires the report test IDs to equal the five-test denominator. Unexpected WPT outcomes do not by themselves fail this evidence gate; incomplete, ambiguous or incorrectly bound evidence does.

## First measured baseline

The first complete non-synthetic run was produced from exact Rarog implementation commit:

`dcd349dd37b0340ec67a2fb8d36b13980e2fd918`

against the pinned WPT commit above on GitHub Actions Ubuntu 24.04 x86_64.

Versioned evidence:

- `wpt/evidence/r6-first-wptreport.json` — unnormalized upstream wptreport content;
- `wpt/evidence/r6-first-evidence.json` — exact denominator/commit/content binding;
- `wpt/evidence/r6-first-dashboard.json` — deterministic normalized machine summary;
- `wpt/evidence/r6-first-dashboard.md` — deterministic human-readable summary.

The canonical report digest is:

`sha256:087d34554cefb38d54d82323a5a8d3419e3a97936fe9b4a2e539e2a2cd6f347f`

The observed selected scope is exactly five tests:

- `/css/selectors/dir-style-01a.html` — **FAIL**, expected PASS;
- `/css/selectors/dir-style-03a.html` — **FAIL**, expected PASS;
- `/html/syntax/parsing/ambiguous-ampersand.html` — **ERROR**, expected OK;
- `/html/syntax/parsing/no-doctype-name.html` — **ERROR**, expected OK;
- `/html/syntax/parsing/zero.html` — **ERROR**, expected OK.

The two reftest failures were produced after real Rarog screenshots were returned to upstream `RefTestImplementation`; their report records distinct test/reference screenshot hashes. This ADR does not infer a standards-engine root cause from that observation alone.

The three HTML testharness errors are explicit capability-boundary results: R6.3 does not yet integrate WPT testharness with the Rarog DOM/script environment. They must remain visible until that capability exists and is measured.

## Reproduction and integrity

The committed historical report remains evidence for the implementation commit that produced it. Later evidence-only commits do not rewrite that embedded Rarog identity.

Protected compatibility-tooling tests reconstruct the evidence envelope and dashboard from the committed selection plus raw report and require exact equality with the committed JSON/Markdown outputs. A later run may have different timing fields or results, but it does not retroactively alter this baseline.

## Consequences

- R6 now has a real, non-synthetic WPT baseline without claiming any selected test passes.
- Failures and unsupported capability remain part of the denominator.
- Upstream wptrunner owns WPT test/result semantics; Rarog owns only the bounded render/unsupported bridge and evidence binding.
- WebDriver/BiDi remains deferred to its dedicated R6 workstream.
- Future WPT improvements can be compared against an immutable measured baseline instead of replacing it.
- Passing the selected-evidence workflow means the measurement is complete and internally consistent, not that Rarog is WPT-conformant.
