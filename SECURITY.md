# Security Policy

Rarog treats Web-controlled content, parser input, script-visible data, resource metadata, dimensions, URLs and platform-facing requests as hostile input.

Rarog is experimental. The current single-process bootstrap engine is **not** a production security boundary and must not be described as sandboxed or safe for arbitrary hostile Web content. Production security claims begin only after the planned multi-process isolation, brokered capabilities and sandbox milestones are implemented and reviewed.

## Supported versions

Rarog has no published production release series yet.

| Version | Security support |
| --- | --- |
| Current `main` | Security fixes accepted |
| Historical commits and milestone snapshots | Not supported |
| Unreleased local or downstream modifications | Maintainer responsibility |

When release builds exist, this policy should move to an explicit latest-release support model rather than silently treating old binaries as supported.

## Reporting a vulnerability

Prefer GitHub private vulnerability reporting when the repository UI makes it available.

Do not publish vulnerability details, exploit material, credentials, tokens, private keys, signing material or other sensitive evidence in a public issue, discussion, pull request, commit message or CI log.

If private vulnerability reporting is unavailable, open only a minimal public issue requesting a private reporting channel. Do not include vulnerability details in that issue.

Include, where practical:

- the affected commit or subsystem;
- minimal reproduction steps;
- expected security impact;
- whether hostile HTML/CSS/URL/script/platform input is required;
- relevant logs or traces with secrets removed.

If testing may expose credentials or privileged host access, stop after collecting the minimum evidence needed to establish the issue.

## Triage process

- Initial acknowledgement target: within 3 business days.
- Initial severity and scope assessment target: within 7 business days.
- Critical and high-impact issues take priority over feature work.
- Valid reports remain private until a fix or mitigation is available and disclosure is appropriate.

These are response targets, not guarantees of a specific remediation date.

## Security scope

In scope:

- HTML, CSS, URL, Fetch and WebIDL parsing/normalization boundaries;
- DOM mutation, event, scheduler, layout, paint and compositor resource-safety failures;
- hostile-input panics, integer/geometry overflow and unbounded resource retention;
- script-runtime and SpiderMonkey adapter isolation, lifetime and unsafe-code defects;
- origin/site identity and future capability/process-boundary correctness;
- Windows platform, input/IME, clipboard, font and GPU adapter boundaries;
- dependency and build-toolchain vulnerabilities introduced by this repository;
- GitHub Actions, dependency resolution, source integrity and future release provenance.

Generally out of scope unless Rarog directly causes or amplifies the issue:

- vulnerabilities in GitHub, operating systems, GPU drivers or third-party infrastructure;
- denial of service that requires unrealistic local resource exhaustion outside a Rarog-controlled budget;
- findings that assume an already fully compromised host without crossing an additional Rarog trust boundary.

## Repository and supply-chain security

Rarog security-sensitive repository policy is:

- changes flow through short-lived pull requests;
- external GitHub Actions are pinned to immutable full commit SHAs;
- workflow containers are pinned by immutable digest;
- `GITHUB_TOKEN` uses explicit least-privilege permissions;
- CI resolves committed Cargo dependencies with `--locked`;
- RustSec audits `Cargo.lock`;
- CodeQL scans Rust and GitHub Actions workflows;
- Semgrep provides an independent SAST layer;
- Gitleaks scans repository history for secrets;
- RustSec is the active dependency-vulnerability merge gate; GitHub Dependency Review remains intentionally disabled until the repository dependency graph is enabled and the action can run reliably;
- Dependabot covers Cargo and GitHub Actions updates;
- Dependabot version updates remain useful independently of Dependency Review; Dependabot alerts/security updates require the GitHub dependency graph/security-analysis feature to be enabled;
- the workspace forbids ordinary unsafe Rust, with the intentionally isolated SpiderMonkey adapter reviewed separately.

Never commit an `.env` file, token, private key, certificate private material, signing key, service credential or other secret.

## Release security

Rarog currently has no release pipeline and no published releases. Artifact attestation, signing credentials and immutable `v*` release-tag rules therefore are not added merely for appearance.

Before the first binary release pipeline is enabled, it must:

- build from an immutable version tag associated with the intended `main` commit;
- protect release tags against update and deletion;
- keep signing material outside the repository;
- use job-scoped write/OIDC permissions only where required;
- produce checksums and GitHub artifact attestations for published binaries where supported;
- refuse replacement of an existing release/tag with different content.
