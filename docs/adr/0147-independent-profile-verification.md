# ADR-0147: Independent canonical profile verification

## Status

Accepted for R6.9 pending exact-head and merged-main verification.

## Context

R6.8 defines the first versioned compatibility profile from two historical evidence sets. Its generator reconstructs source evidence and emits deterministic JSON and Markdown.

A generator successfully reproducing its own output is necessary but insufficient before cryptographic signing. A second verifier must independently validate the profile's content identity and source bindings.

## Decision

`scripts/compat_profile_verify.py` is a separate verifier that **does not import or call `scripts/compat_profile.py`**.

For profile schema version 1/revision 1 it requires exactly the two published historical evidence identities, separately:

- WPT: source selection, exact upstream commit, raw WPT report, committed evidence envelope, normalized dashboard and Markdown; the full selected denominator, expectation metadata, observed FAIL/ERROR and historical engine commit are retained.
- Real-Web: exact corpus, raw attempt bundle, normalized scenario baseline and Markdown; origin/dependency outcomes, completed observations and historical engine commit are retained.

The verifier independently parses and binds the profile entry data, checks exact artifact identities, canonical input digests and source-normalizer reproduction, and re-renders the profile Markdown without invoking the profile generator. It intentionally reuses the pre-existing, separately tested **source-level** WPT and real-Web evidence normalizers; independence here applies to profile reconstruction and byte identity rather than claiming to duplicate every underlying standards/parser contract.

Every artifact path is constrained to its expected subtree, must be normalized and is checked for symlink traversal. JSON duplicate keys, NaN/infinite values, unknown profile keys, boolean-versus-integer substitutions, unsorted/duplicate evidence, changed sources and malformed identities fail closed.

## Canonical payload bytes

The profile payload to be content-addressed is:

```python
json.dumps(profile, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")
```

The repository's human-inspectable `compatibility/profile.json` is separately required to equal the same validated object serialized as indented, sorted UTF-8 JSON with a final newline. Its Markdown must match the independent renderer byte-for-byte.

The verifier emits a small deterministic verification envelope with:

- `schema_version` and `profile_revision`;
- `profile_sha256` of the canonical payload **bytes**, not the pretty file;
- byte length, evidence count and sorted evidence IDs;
- exact set of historical measured Rarog commits.

The first published envelope is `compatibility/verification.json`. The canonical profile payload is 2097 bytes, with digest `sha256:adc16fa371c260b49b8f9fc476e91fdd304c1a1ef3ceb10712963bdb41731c66`. The envelope is independently regenerated during protected tests.

This SHA-256 is **content identity**, not a signature, trust assertion, current-engine measurement or conformance score. Any future profile revision requires an explicit verifier schema change, not a silent acceptance of unknown evidence.

## Consequences

- A corrupted source record cannot enter a signed candidate profile merely because the profile JSON looks well formed.
- Regeneration, independent verification and Markdown production have distinct implementation boundaries.
- The existing WPT FAIL/ERROR and real-Web completed-observation states remain unchanged.
- The next R6 slice may sign only the independently verified canonical bytes under a separately approved key/provenance policy.
