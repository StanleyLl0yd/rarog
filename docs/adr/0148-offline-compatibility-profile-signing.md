# ADR-0148: Offline Ed25519 compatibility-profile signing

## Status

Proposed for R6.10; validation gates and permissioned production key provisioning remain prerequisites for publication.

## Context

ADR-0147 establishes strict independent revalidation of the R6 historical evidence, an exact compact sorted UTF-8 profile payload, and its SHA-256 identity. A digest by itself authenticates neither signer nor provenance.

## Decision

The signature is over **exactly** the compact canonical UTF-8 profile bytes reconstructed from the independently verified `compatibility/profile.json` (ADR-0147), not the pretty JSON file, Markdown, arbitrary archive, measured engine commit or checkout revision.

Use standardized **Ed25519** (RFC 8032), **OpenSSL 3** `pkeyutl -rawin` signing/verification, and standard RFC 8410 SubjectPublicKeyInfo (SPKI) DER public keys. Python only performs strict envelope parsing, canonicalization, digest calculation, trust lookup and subprocess orchestration; it does not implement public-key cryptography. OpenSSL execution is local/offline. Windows-2025 and Ubuntu hosted runners provide Python/OpenSSL; explicitly exercise both in protected CI.

The v1 signature envelope is strictly sorted, pretty UTF-8 JSON (two-space indentation and trailing LF), with exactly:

- `schema_version: 1`
- `algorithm: "Ed25519"`
- `key_id: "sha256:" + SHA-256 of the 44-byte canonical Ed25519 SPKI DER`
- `payload_sha256` from the independent verifier
- `signature_base64` (canonical padded base64 of the 64-byte Ed25519 signature)

The **external release-specific pinned trust root** is strict v1 JSON containing the exact `profile_sha256`, the required `expected_key_id` and non-empty, sorted, unique `keys` entries. Each entry contains exactly `algorithm`, `key_id`, `public_key_spki_der_base64`, `status` (`active` or `revoked`). The verifier does not trust a public key embedded in an untrusted signature envelope. It validates canonical SPKI representation, SPKI fingerprint, signed digest, exact profile content, trusted key status and cryptographic signature, failing closed.

Signing requires a private key path **outside the repository checkout**; output may never overwrite that key, the profile or trust root. No private key is generated, exported, displayed or stored by CI, GitHub or the signing script. Tests generate isolated ephemeral keys outside the checkout and delete them; test results do not imply a production signature.

## Rotation, revocation and provenance

A designated release operator independently provisions and controls the private key and publishes its public SPKI/fingerprint as part of an explicitly pinned, reviewed trust root for that profile/release. Rotating a signer requires publishing a reviewed trust-root revision with the new `expected_key_id`; signatures from other active keys remain rejected for that profile/release. Removing or marking a key `revoked` causes its signatures to fail. Historical signatures require the independently pinned historical release trust root; no timestamp-based grandfathering is inferred. Consumers must explicitly pin the trusted release/root revision; a mutable/downloaded trust file accompanying a signature is not an independently established trust anchor.

This format intentionally contains no unverifiable timestamp, online certificate/PKI assertion, automatic key discovery or self-attested signer identity. The signature authenticates the canonical historical evidence index **only relative to the externally pinned public-key trust root**. It is not an engine binary signature, certification, fresh measurement, compatibility score or a claim of a common measured commit.

## Publication gate

Do **not** commit `compatibility/profile.sig.json`, a production trust root or any claim that the published profile is signed until the release operator has permissioned/provisioned the real signing key, independently pinned the public key, locally produced the real signature and passed offline verification. The protected synthetic/ephemeral test matrix checks only the signing implementation and its failure behavior. R6.10 cannot be accepted on test-fixture signatures alone.
