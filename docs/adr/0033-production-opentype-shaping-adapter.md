# ADR-0033: Production OpenType shaping adapter

Status: Accepted

## Context

R1 already owns the text-segmentation and shaping request contracts in `rarog-layout`: scalar-index `TextRange`, bidi level/direction, script, language, OpenType features, variation coordinates, selected `FontFaceId`, positioned glyphs and font metrics. The deterministic `FixedTextShaper` proves those contracts but cannot provide production glyph selection, ligatures, kerning, contextual shaping or variable-font positioning.

A production backend must not move third-party shaping policy into layout, must preserve Rarog's scalar-index source ranges even though OpenType shapers expose UTF-8 byte clusters, must remain usable on non-Windows hosts, and must bound retained font bytes because font data can originate outside the engine trust boundary.

## Decision

Add `rarog-text-opentype` as a narrow adapter crate implementing the existing `ShapingBackend` contract with HarfRust.

The crate owns a bounded registry of validated OpenType font bytes keyed by Rarog `FontFaceId`. Registration validates the face index and requested pixel size before retention, limits face count, per-face bytes and total retained bytes, and caches HarfRust `ShaperData` without exposing HarfRust types to layout.

The fallible `try_shape_run` path is the production integration boundary. It maps Rarog bidi direction, script, language, features and variation coordinates into HarfRust, shapes at 1/64-pixel positional precision, and converts HarfRust UTF-8 byte cluster offsets back into Rarog scalar-index `TextRange` values. Glyph IDs, horizontal advances and offsets are returned through Rarog-owned positioned-glyph types; line metrics remain supplied by the selected Rarog `FontFace`, so the upcoming platform font adapter can keep metrics and font selection coherent.

The existing infallible `ShapingBackend::shape_run` implementation delegates to `try_shape_run` and falls back to `FixedTextShaper` when the production registry or font data is unavailable. This preserves the existing no-panic trait boundary and deterministic bootstrap behavior while allowing engine/platform code to use the fallible API where setup errors must be surfaced explicitly.

HarfRust is kept in this leaf adapter rather than `rarog-layout`. This keeps the dependency replaceable, allows the core layout contracts to stay platform- and shaper-neutral, and avoids coupling the later Windows font-discovery layer to a specific shaping implementation.

## Consequences

- Rarog gains real OpenType glyph selection and positioning without changing layout-owned shaping contracts.
- UTF-8 byte clusters cannot leak into fragment/source identity; they are translated back to scalar ranges at the adapter boundary.
- Font memory is explicitly bounded and accounted.
- Windows font discovery can register system font bytes and matching metrics without placing Windows APIs in layout.
- The deterministic fixed shaper remains available for bootstrap snapshots and as a defensive fallback; it is not the production success path.
- The production adapter adds HarfRust as a focused dependency. HarfRust 0.13.x matches Rarog's Rust 1.85 MSRV and is maintained as the successor to the archived Rustybuzz project.
