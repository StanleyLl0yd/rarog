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

## Interpretation rules

- WPT and real-Web evidence remain separate kinds.
- WPT FAIL/ERROR states remain visible.
- Real-Web scenario outcomes and external dependency states remain visible.
- Evidence measured on different Rarog commits is never relabeled as one common-current measurement.
- No aggregate percentage, score, rank or cross-kind status is produced.
- Profile revision identifies the profile schema/content lineage, not an engine measurement revision.
- Signing is deliberately a later R6 slice after profile reproduction/content identity is established.
