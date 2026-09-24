# ADR-0143: Versioned real-Web corpus and external-dependency boundary

## Status

Accepted for the R6.5 real-Web contract slice.

## Context

WPT gives R6 standards-focused, versioned inputs. Real-Web compatibility has a different evidence problem: public sites are external systems whose DNS, certificates, HTTP responses, redirects, markup, subresources, APIs and availability can change without any Rarog change.

A useful corpus therefore cannot treat every failed navigation as an engine failure, nor can it silently refetch a page and call new bytes the same captured input. It also must not turn compatibility qualification into an unbounded crawler, authenticated browser session or site-specific special-case mechanism.

## Decision

Rarog stores the real-Web scenario contract in `real-web/corpus.json` and validates it with `scripts/real_web_corpus.py`.

### Input modes

Each scenario has exactly one input mode.

**`live-external`**

- identifies a canonical public HTTPS source URL;
- carries no content digest;
- explicitly depends on declared public external origins;
- remains mutable and can become unavailable independently of Rarog.

**`captured-versioned`**

- identifies a normalized repository file under `real-web/captures/`;
- binds it to exact SHA-256 and an allowed media type;
- rejects symlinks;
- requires an empty external-dependency list in schema v1, making the captured input offline-reproducible.

Changing captured bytes requires an explicit manifest digest change. Captured content is never silently refreshed in place.

### Network boundary

Initial live scenarios:

- use HTTPS only;
- contain no URL userinfo or fragments;
- cannot target localhost, wildcard hosts, private/loopback/link-local/multicast/reserved/unspecified IP addresses;
- declare every allowed origin explicitly;
- declare the source origin exactly once as a required `primary-document`.

The manifest validator rejects unsafe literal host targets. A later network executor must additionally validate resolved addresses before connecting and on redirects so DNS resolution cannot turn an allowed public hostname into a private/link-local target after contract validation.

Dependency roles are closed to `primary-document`, `subresource` and `api`.

### Interaction and resource boundary

Schema version 1 permits only:

- `load-input`;
- `wait-for-idle` with a bounded timeout.

Scenarios declare bounded viewport/device scale, overall timeout, per-response bytes, total bytes and subresource count. Observations are selected from a closed set: document title, final URL, render completion and screenshot.

Free-form script/action payloads are deliberately absent.

### Privacy and side-effect policy

The root corpus policy requires the following to remain forbidden:

- credentials;
- authenticated/private content;
- payments;
- destructive actions;
- persistent identity;
- undeclared network origins.

A later schema revision is required to broaden any of these boundaries.

### Outcome taxonomy

Later real-Web execution must preserve exactly:

- `completed-observation`;
- `engine-failure`;
- `external-content-drift`;
- `external-unavailable`;
- `unsupported-capability`.

In particular, external unavailability and external content drift are not Rarog regressions.

### Initial scenario

The first contract contains one small public, non-authenticated live scenario, `rfc9110-html`, against the RFC Editor HTML representation of RFC 9110. R6.5 selects and validates the scenario only; it does not execute it or publish a compatibility outcome.

## Consequences

- Real-Web inputs have an explicit mutability model before measurement begins.
- Captured inputs can be reproduced offline and content drift is reviewable.
- Live service failures stay distinguishable from engine failures.
- The initial corpus cannot access credentials, authenticated/private pages, payments or destructive actions.
- Site-specific behavior cannot be hidden in free-form scenario scripts.
- A future execution slice can emit evidence against stable scenario IDs without changing the corpus contract in response to outcomes.
