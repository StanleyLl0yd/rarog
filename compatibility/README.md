# Compatibility profiles

Rarog R6 compatibility profiles collect references to already-versioned, independently reproducible evidence. A profile is an evidence index, not a browser score and not a claim that every measurement was produced by the repository commit that contains the profile.

## Canonical R6 profile

- `profile.json` is the machine-readable canonical payload.
- `profile.md` is generated from the same validated payload.
- `scripts/compat_profile.py` reconstructs every included evidence set before accepting it.

The initial profile contains two historical evidence sets:

1. the first selected WPT baseline;
2. the first bounded real-Web baseline.

Each entry retains its own exact measured Rarog commit, upstream/corpus identity, raw evidence digest, normalized evidence digest, measured environment and exact scope identity.

## Reproduce

```text
python3 scripts/compat_profile.py \
  --root . \
  --json-out /tmp/rarog-profile.json \
  --markdown-out /tmp/rarog-profile.md

cmp /tmp/rarog-profile.json compatibility/profile.json
cmp /tmp/rarog-profile.md compatibility/profile.md
```

The protected compatibility-evidence test suite performs the same reconstruction automatically.

## Independent verification and content identity

R6.9 adds an independent verifier that does not import or call the profile generator:

```text
python3 scripts/compat_profile_verify.py --root . --json-out /tmp/rarog-profile-verification.json
```

It independently binds the historical source evidence, validates the committed profile and Markdown byte-for-byte, and emits the canonical payload SHA-256. The signed-payload candidate uses compact sorted UTF-8 JSON with no ASCII escaping as specified in ADR-0147; the pretty JSON file is a human-readable view of the same data.

The committed `verification.json` is reproduced byte-for-byte by the independent test suite. For revision 1, the compact canonical profile is **2097 UTF-8 bytes** and has digest:

`sha256:adc16fa371c260b49b8f9fc476e91fdd304c1a1ef3ceb10712963bdb41731c66`

The digest is **not a signature**, a common measured engine version or an aggregate compatibility score. Signing remains a separate, later step.

## Offline signing (R6.10 in progress)

`scripts/compat_profile_signature.py` accepts **only** the exact independently verified compact profile payload from ADR-0147 and signs it with Ed25519 via local OpenSSL 3. It neither generates keys nor signs unverified evidence.

A release operator must first provision an Ed25519 private key **outside** the checkout and independently publish/pin a reviewed public trust root. The trust root is strict canonical JSON with `schema_version: 1` and a non-empty sorted `keys` array; each key has `algorithm: "Ed25519"`, `key_id` (SHA-256 of SPKI DER), `public_key_spki_der_base64` (canonical SPKI DER), and `status: "active"` or `"revoked"`.

After that provisioning, the release operator can sign locally:

```text
python3 scripts/compat_profile_signature.py sign \
  --root . --trust /trusted/release-trust.json \
  --private-key /secure/off-repository/ed25519.pem \
  --signature compatibility/profile.sig.json
```

Anyone with the separately pinned trust root can verify **offline, without private keys**:

```text
python3 scripts/compat_profile_signature.py verify \
  --root . --trust /trusted/release-trust.json \
  --signature compatibility/profile.sig.json
```

Neither a production trust root nor a real signature has been provisioned/published by R6.10 tooling development. Ephemeral CI keys and fixtures only test signing/verification behavior. Do not infer a trusted signature or an engine compatibility claim from those tests. See ADR-0148 for trust provenance, key rotation and revocation.

## Interpretation rules

- WPT and real-Web evidence remain separate kinds.
- WPT FAIL/ERROR states remain visible.
- Real-Web scenario outcomes and external dependency states remain visible.
- Evidence measured on different Rarog commits is never relabeled as one common-current measurement.
- No aggregate percentage, score, rank or cross-kind status is produced.
- Profile revision identifies the profile schema/content lineage, not an engine measurement revision.
- Signing is deliberately a later R6 slice after profile reproduction/content identity is established.
