# R1 focused WPT preflight

Rarog starts Web Platform Test work in R1 instead of deferring compatibility validation until a later dashboard milestone.

`r1-focus.txt` is the versioned initial scope. Entries are upstream WPT directory scopes, not claims that every test in those directories is already supported. R1 feature work should progressively turn those scopes into executable, pinned test selections as the relevant parser/style/layout behavior lands.

## Policy

- Use the upstream `web-platform-tests/wpt` repository as the source of truth.
- Record the upstream WPT commit whenever results are published or used as a merge gate.
- Prefer the narrowest relevant test paths for feature PRs; do not report an entire directory as passing from a partial run.
- Keep upstream expectations separate from Rarog implementation code.
- A newly supported R1 standards feature should link to at least one relevant focused WPT case or document why no suitable upstream case exists yet.
- Known failures must remain explicit; do not silently remove failing paths from the focus list to improve percentages.

## Initial R1 areas

The initial focus covers HTML parsing plus CSS cascade/selectors/display/box/text behavior because those map directly to the R1 Flame roadmap. Networking, JavaScript, accessibility, browser UI, and compositor suites remain outside this initial subset.

`crates/rarog-engine/tests/wpt_preflight.rs` keeps the focus manifest non-empty and directory-scoped in normal CI. Actual WPT execution is added incrementally with R1 standards work, using pinned upstream revisions and concrete test paths.
