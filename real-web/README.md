# R6 real-Web corpus

This directory defines bounded real-Web scenario contracts. It does not itself contain compatibility results.

## Why a contract exists before execution

Public Web pages and services can change independently of Rarog. DNS, TLS, HTTP availability, redirects, content, subresources and APIs are all external state. R6 therefore separates:

- **captured-versioned** inputs, which are repository files bound to SHA-256 and require no external network dependencies;
- **live-external** inputs, which identify a public HTTPS resource but explicitly remain mutable and dependent on declared external origins.

A live service outage or external content change must not be reported as an engine regression.

## Initial corpus

`corpus.json` starts with one deliberately bounded live scenario:

- `rfc9110-html`
- `https://www.rfc-editor.org/rfc/rfc9110.html`
- one required primary-document origin;
- viewport 1280x720 at scale 1;
- `load-input` then a bounded `wait-for-idle`;
- title/final-URL/render/screenshot observations;
- explicit per-response, total-byte, subresource and overall timeout budgets.

This is scenario selection only. R6.5 does not execute the scenario and does not publish a real-Web result.

## Validation

```text
python3 scripts/real_web_corpus.py \
  --manifest real-web/corpus.json \
  --root .
```

The validator fails closed on, among other things:

- duplicate/ambiguous JSON;
- unknown schema keys, actions, observations or dependency roles;
- non-HTTPS, credential-bearing, fragment-bearing or unsafe/private network targets;
- wildcard/non-canonical origins;
- unsorted or duplicate scenario/dependency/observation identities;
- unbounded viewports, timeouts, response sizes, total bytes or subresource counts;
- live inputs that pretend to have immutable capture identity;
- captured inputs outside `real-web/captures/`, symlink captures, digest mismatch or unsupported media types;
- captured inputs that still require external network dependencies;
- a live scenario that does not declare its source origin as one required primary document.

## Privacy and safety boundary

Schema version 1 forbids:

- credentials;
- authenticated/private content;
- payments;
- destructive actions;
- persistent user identity;
- undeclared network origins.

The initial action language is intentionally small: `load-input` and `wait-for-idle`. Interaction actions require a reviewed schema extension rather than free-form scripting.

## Outcome taxonomy for later execution

When execution is added in a later slice, results must preserve these categories:

- `completed-observation`
- `engine-failure`
- `external-content-drift`
- `external-unavailable`
- `unsupported-capability`

The two external categories are not Rarog regressions.

See ADR-0143 and `docs/R6-EXIT.md`.
