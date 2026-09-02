# ADR-0034: Windows system-font adapter

Status: Accepted

## Context

R1 now has a production OpenType shaping adapter behind Rarog-owned `ShapingBackend` and `ShapingRequest` contracts. That adapter deliberately does not discover fonts or call platform APIs: it consumes validated font bytes, a collection face index, pixel size, and Rarog-owned metrics/identity supplied by a higher platform layer.

The first production target is Windows 10/11. Rarog therefore needs a first system-font adapter that can perform CSS-like family/style/weight/stretch matching, retrieve the selected OpenType bytes for shaping, and expose normalized font metrics without allowing DirectWrite or a third-party font library to leak into layout, shaping requests, or engine-wide data structures.

The platform boundary is shared by embedders and future operating-system adapters, so it must remain `Send + Sync`, validate requests before platform work, and have a deterministic unsupported-target behavior on non-Windows builds.

## Decision

Add Rarog-owned font platform contracts to `rarog-platform`:

- `PlatformFontFamily` for named and CSS generic families;
- `PlatformFontStyle` plus numeric weight and stretch;
- `PlatformFontRequest` with validated family list, style properties, and pixel size;
- `PlatformFontMetrics` and `PlatformFontProperties` containing only normalized Rarog-owned values;
- `ResolvedPlatformFont` containing owned OpenType bytes, collection face index, selected names/properties, requested pixel size, and pixel-scaled metrics;
- `PlatformFontService`, exposed optionally through `PlatformHost::font_service`.

Implement the first adapter in `rarog-platform-windows` as `WindowsFontService`. On Windows it uses `font-kit` 0.14.x as a narrow wrapper around the DirectWrite system font source. `font-kit` remains a target-specific dependency of the Windows platform crate and its types never cross the platform boundary.

`WindowsFontService` is intentionally stateless. Each resolution opens the Windows system font source for that operation instead of retaining DirectWrite/font-kit objects inside `WindowsPlatformHost`. This keeps COM/native object lifetimes and any platform threading constraints out of the shared `PlatformHost: Send + Sync` contract. A bounded cache may be added later behind the same service boundary if profiling justifies it.

Font metrics are converted from font units into pixels using `size_px / units_per_em`. Rarog normalizes descent to a non-negative distance below the baseline and clamps negative line gap to zero. Invalid, non-finite, or zero-em metrics are rejected before they enter layout-owned structures.

The selected font's raw bytes and original collection face index are returned together. This is required because a DirectWrite-selected face may live inside TTC/OTC collection data; the production OpenType shaper must register the same face rather than assuming index zero.

On non-Windows targets the Windows adapter compiles without `font-kit` and returns `PlatformFontError::UnsupportedTarget`. A constructed Windows host advertises the `FontText` platform capability and exposes its font service; the null host continues to expose no platform font service.

## Consequences

- DirectWrite and `font-kit` stay isolated to the Windows platform leaf crate.
- Layout and the HarfRust adapter receive only Rarog-owned values and OpenType bytes.
- CSS-like system font matching is available without making the shaping crate platform-aware.
- Collection face indices survive discovery and can be passed unchanged to HarfRust.
- Pixel metrics have one sign/scale convention across platform implementations.
- The shared platform crate gains a stable seam for future macOS/Linux font adapters without changing layout contracts.
- The first implementation favors simple lifetime/threading behavior over caching; repeated resolution may cost more until measurements justify a bounded cache.
- System font availability is inherently machine-dependent, so tests assert structural invariants and use a Windows-default family with a generic fallback rather than snapshotting a particular font file or glyph geometry.
