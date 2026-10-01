# ADR-0145: First measured real-Web baseline

## Status

Accepted for the R6.7 compatibility-measurement slice.

## Context

R6.5 defined a bounded real-Web corpus and R6.6 defined deterministic result classification, but neither executed a live scenario.

The initial corpus contains one public unauthenticated live scenario, `rfc9110-html`, targeting the RFC 9110 HTML publication. A live measurement has two independent failure domains:

- external network/service/content state;
- Rarog navigation/render/observation behavior.

Using an ordinary redirecting HTTP client would weaken that separation because DNS changes, unsafe redirect targets and private-address resolution could occur below the evidence layer.

## Decision

R6.7 adds a qualification-only live executor and a test-only Rarog renderer.

For every live request/redirect hop the executor:

- requires a canonical HTTPS URL whose origin is declared by the corpus;
- resolves DNS immediately before connection;
- rejects any private, loopback, link-local, multicast, reserved or unspecified resolved address;
- connects the TCP socket to an already validated numeric IP while retaining the original hostname for TLS SNI and certificate verification;
- re-resolves and revalidates each redirect target;
- enforces a bounded redirect count, per-response bytes, aggregate response bytes and scenario deadline;
- sends no credentials, cookies or persistent identity.

Unsafe resolution is treated as a hard qualification-policy failure rather than contacting the address.

External DNS/TLS/HTTP/redirect/resource/timeout outcomes remain external evidence. Once the required primary document is available, render/navigation failures are allowed to become engine outcomes.

The test-only Rust renderer uses the existing `render_html` pipeline. The document title is read from the parsed Rarog DOM, not from an independent HTML parser. The screenshot observation is SHA-256 over the deterministic PPM bytes emitted by the Rarog framebuffer.

The full raw execution is then passed through the already-merged R6.6 result normalizer. A corpus-level binder requires exactly one attempt for every corpus scenario in exact manifest order and content-addresses the raw execution. Missing/extra/reordered scenarios fail closed.

## First measured baseline

The first successful live qualification run was produced from exact Rarog implementation commit:

`2301370b6ccce06a8839a94a75dfeee12e3a3499`

on GitHub Actions Ubuntu 24.04 x86_64.

Corpus identity:

- revision: `1`;
- normalized corpus SHA-256: `sha256:835751b5e67c10ac4f77c838d4e4496724597814c9917f58b1358a7d076df203`;
- measured denominator: exactly one scenario, `rfc9110-html`.

Raw execution identity:

`sha256:daa3bf52900393e3755b903f0ba2fef9a310b3c3294c84869908ee27a1acf2b8`

Observed external primary-document evidence:

- origin: `https://www.rfc-editor.org`;
- HTTP status: `200`;
- observed content SHA-256: `sha256:d431760660ea44e130f6e919dab216df2d0b3a490567a98089267523368fe1e5`;
- dependency state: `available`.

Observed Rarog evidence:

- outcome: `completed-observation / completed`;
- document title: `RFC 9110: HTTP Semantics`;
- final URL: `https://www.rfc-editor.org/rfc/rfc9110.html`;
- render completion: `true`;
- screenshot PPM SHA-256: `sha256:376a97100a0ae7fd1027bc5054c03e08c21d575347baf99443627ec0def0708a`.

Versioned evidence is stored under `real-web/evidence/`.

## Reproduction and mutability

The committed raw execution deterministically reproduces the committed normalized baseline JSON/Markdown and is tested as such.

The live RFC resource is intentionally **not** copied into the repository and remains externally mutable. A later live run can therefore resolve to different public addresses or content bytes and must produce new evidence rather than rewriting this historical baseline. The baseline proves what was observed for the exact implementation/corpus/platform/run; it does not freeze or endorse the external site.

A successful `completed-observation` for this one scenario is not a general-Web compatibility result and carries no compatibility percentage.

## Consequences

- R6 now has one real measured real-Web scenario in addition to the WPT baseline.
- Network/service failures remain distinguishable from Rarog engine failures.
- The qualification runner does not introduce a production HTTP client, crawler or browser automation API.
- No credentials or authenticated/private resources are used.
- Future corpus growth remains an explicit reviewed denominator change.
- WebDriver/BiDi and higher-level Web-app interaction remain separate R6 workstreams.
