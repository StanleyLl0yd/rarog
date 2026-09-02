# ADR-0033: Production OpenType shaper behind the Rarog boundary

**Status:** Accepted

## Context

Rarog already splits text into grapheme-safe bidi, font and script shaping requests and exposes backend-neutral glyph IDs, advances, offsets and source ranges. The R0 `FixedTextShaper` deliberately ignores OpenType semantics. R1 needs a real production shaping implementation without leaking a third-party API into layout callers or coupling shaping to Windows font discovery.

## Decision

Use HarfRust behind the existing Rarog-owned `ShapingBackend` contract. Rarog owns immutable `OpenTypeFontData` containing font bytes, collection face index and pixels-per-em, while `OpenTypeShaper` maps those records to existing `FontFaceId` values. HarfRust types remain implementation details.

Make the shaping backend contract explicitly fallible. Missing faces/font data, invalid font data, invalid language metadata and invalid source clusters are errors rather than synthetic fallback output or panics. The fixed bootstrap backend returns successful deterministic output through the same result boundary.

The adapter forwards direction, script, language, whole-run OpenType feature settings and variation coordinates from `ShapingRequest`. Input characters are added with global Rarog character-index cluster values. Output cluster starts are sorted independently of visual glyph order and converted back into logical `TextRange` values, preserving source ownership for ligatures and RTL shaping.

Use a fixed 1/64-pixel position scale for the HarfRust adapter. Font metrics remain owned by the resolved Rarog `FontFace`; the upcoming platform font adapter is responsible for supplying production metrics and font data consistently.

## Consequences

- Complex OpenType substitution and positioning can be exercised now without changing segmentation or fragment identity.
- Feature and variation metadata cross a measured production backend instead of being ignored by every implementation.
- The backend can fail explicitly on malformed or absent font data.
- No system-font lookup, Windows API dependency or default-font policy enters the portable layout crate.
- Default Web layout remains on the fixed bootstrap geometry until platform font discovery is connected.
