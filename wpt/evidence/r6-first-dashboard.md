# Rarog WPT compatibility dashboard

- Rarog commit: `dcd349dd37b0340ec67a2fb8d36b13980e2fd918`
- Upstream WPT commit: `a83afd4402cffdc876508fe9a47f916d4136099f`
- Platform: `github-actions-ubuntu-24.04`
- Source reports: 1

## Measured scope

| Level | Measured | With expectation metadata | Unexpected |
| --- | ---: | ---: | ---: |
| Tests | 5 | 5 | 5 |
| Subtests | 0 | 0 | 0 |

Only records present in the supplied WPT reports are measured. This dashboard does not infer results for unmeasured tests or directories and does not claim general Web compatibility.

## Test statuses

| Status | Count |
| --- | ---: |
| ERROR | 3 |
| FAIL | 2 |

## Subtest statuses

| Status | Count |
| --- | ---: |
| *(none)* | 0 |

## Test outcomes

| Test | Observed | Expected | Unexpected | Subtests |
| --- | --- | --- | --- | ---: |
| `/css/selectors/dir-style-01a.html` | FAIL | PASS | yes | 0 |
| `/css/selectors/dir-style-03a.html` | FAIL | PASS | yes | 0 |
| `/html/syntax/parsing/ambiguous-ampersand.html` | ERROR | OK | yes | 0 |
| `/html/syntax/parsing/no-doctype-name.html` | ERROR | OK | yes | 0 |
| `/html/syntax/parsing/zero.html` | ERROR | OK | yes | 0 |
