# ADR-0030: Windows platform host boundary

## Status

Accepted.

## Context

R0 has a host-facing `Engine`/`View` API, while Windows 10/11 is the first production target. The remaining platform work must not make DOM, HTML, CSS, layout, paint, or embedder contracts depend on Win32, WinRT, DirectWrite, Direct3D, or other Windows-specific types.

## Decision

Introduce two explicit crates:

- `rarog-platform` owns the platform-neutral `PlatformHost` capability contract used by `rarog-engine`;
- `rarog-platform-windows` is the first target-specific host boundary and may later own Windows API adapters.

`EngineBuilder` accepts a `PlatformHost` and defaults to `NullPlatformHost`. Engine core depends only on `rarog-platform`; it never depends on `rarog-platform-windows`.

R0 exposes capability slots for window/events, font/text, input/IME, accessibility, sandbox/process, and GPU/compositor integration. The Windows host reports those capabilities as unavailable until real adapters are implemented. It must not advertise placeholder services as production-ready.

`WindowsPlatformHost::try_new` succeeds only when compiled for Windows. The crate still compiles in the Linux portability lane so target-specific code remains structurally isolated and workspace-wide checks stay meaningful.

## Consequences

- Windows-specific dependencies can be added later without leaking into engine-core crates.
- Embedders can inspect the active host name and advertised platform capabilities through `Engine`.
- Linux CI can compile the Windows boundary without pretending the Windows host is usable there.
- Concrete window, font, IME, accessibility, sandbox, and GPU adapters remain separate implementation milestones.
- The platform boundary is capability-oriented and does not imply a lowest-common-denominator implementation across operating systems.
