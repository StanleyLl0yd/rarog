# ADR-0144: Deterministic real-Web result and failure classification

## Status

Accepted for the R6.6 compatibility-evidence slice.

## Context

R6.5 defines the versioned real-Web corpus before execution. A result format must preserve the reason a scenario did not complete. A DNS outage, TLS failure, upstream HTTP error, redirect-policy rejection or changed external document is not equivalent to a Rarog render/script failure.

The result boundary also must not make missing evidence disappear. Every declared observation and every declared external dependency needs an explicit record, even when it was not observed or attempted.

## Decision

`scripts/real_web_result.py` normalizes one execution attempt against an already validated `real-web/corpus.json`.

Every canonical result is bound to:

- an exact lowercase 40-hex Rarog commit;
- the SHA-256 of the normalized corpus contract and its corpus revision;
- one exact corpus scenario ID;
- the scenario's source/input identity;
- an explicit measured OS, architecture and environment label;
- one result record for every declared external dependency;
- one result record for every declared observation.

### Outcome model

The five R6.5 corpus categories remain authoritative:

- `completed-observation`;
- `engine-failure`;
- `external-content-drift`;
- `external-unavailable`;
- `unsupported-capability`.

Each category has a closed detail set rather than a score or ordering.

`engine-failure` can identify navigation, render, script, resource-limit or timeout-limit failure.

`external-unavailable` can identify DNS, TLS, HTTP, redirect-policy, resource-limit or timeout-limit failure.

The normalizer rejects an `engine-failure` when a required external dependency is not available. Conversely, an external DNS/TLS/HTTP/redirect/limit result must have a matching required dependency state. External content drift requires a matching required dependency record with distinct expected and observed content digests.

### Dependency evidence

Live dependency records preserve the exact declared origin/role ordering and add only bounded evidence:

- required/optional status inherited from the corpus;
- closed dependency state;
- sorted canonical public resolved IP addresses;
- bounded HTTP status where applicable;
- SHA-256 observed content identity where applicable;
- SHA-256 expected content identity only for explicit drift.

Private, loopback, link-local, multicast, reserved and unspecified resolved addresses are rejected by the evidence normalizer.

A DNS failure cannot claim resolved addresses or HTTP/content evidence. A TLS failure requires public resolved addresses but no HTTP/content identity. HTTP unavailability records an HTTP 4xx/5xx response. Redirect-policy violation records a 3xx without accepting content. Available content requires public addresses, HTTP 2xx and an observed SHA-256. Limit outcomes remain explicit dependency states rather than being silently collapsed into a generic failure.

Captured-versioned scenarios have no dependency result records and cannot emit external-unavailable/content-drift outcomes in schema v1.

### Observation evidence

Every observation declared by the scenario appears exactly once and in corpus order. Its state is either `observed` or `not-observed`.

Observed values are type checked:

- document title: bounded UTF-8 text;
- final URL: canonical public HTTPS URL;
- render completion: boolean;
- screenshot: SHA-256 identity.

A completed scenario requires every declared observation to be present and observed. Missing observations therefore cannot be hidden by omitting records.

### Unsupported capability

An unsupported-capability outcome requires required external dependencies to remain `not-attempted`. This prevents a failed network attempt from being relabeled as an unsupported Rarog feature.

### Output

The normalizer emits deterministic JSON and Markdown from the same normalized record. The Markdown explicitly describes one bounded scenario result and does not emit a general-Web compatibility percentage or ranking.

R6.6 defines and tests the capture/classification contract only. It does not perform live network execution. The first measured corpus baseline is separate work.

## Consequences

- External-service failures stay distinct from Rarog failures.
- Required dependency failures cannot be hidden behind an engine-failure label.
- Missing observation/dependency evidence fails closed.
- Captured inputs remain offline evidence.
- Future live execution has a canonical result format before network behavior is introduced.
- A result record is reproducible from the same corpus contract and raw attempt, but it remains evidence for one scenario/run rather than a general compatibility score.
