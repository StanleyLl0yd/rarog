# ADR-0023: Shaping request metadata

## Status

Accepted.

## Context

The shaping backend boundary can already consume a resolved font face and bidi-safe shaping run, but a production OpenType implementation also needs script, language, feature settings, and variable-font coordinates. Encoding those as backend-specific parameters would leak shaping policy across the layout boundary.

## Decision

Introduce `ShapingRequest`, which owns one existing `ShapingRun` plus `ShapingScript`, `LanguageTag`, a list of `OpenTypeFeature` settings, and `VariationCoordinate` values addressed by four-byte `OpenTypeTag`s. Bootstrap requests split existing bidi×font shaping runs again at grapheme-safe script changes, infer script deterministically for each resulting source range, default language to `und`, and carry no features or variations. `ShapingBackend::shape_run` consumes the request rather than a bare run.

The R0 fixed backend accepts the complete request while intentionally ignoring OpenType behavior so current deterministic geometry does not change.

## Consequences

A production OpenType backend can receive the metadata it needs without changing bidi segmentation, font fallback, source mapping, line layout, fragmentation, or paint identity. Full Unicode Script data, BCP 47 validation/canonicalization, script/language inheritance from CSS/DOM, feature ranges, font-specific axis validation, and platform font discovery remain future work.
