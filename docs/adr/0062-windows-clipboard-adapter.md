# ADR-0062: Windows clipboard adapter boundary

Status: Accepted

## Context

R2 requires a Windows clipboard adapter behind the Rarog-owned `PlatformClipboardService` contract. The engine contract intentionally exposes only bounded plain text. Native clipboard ownership, global handles, Win32 clipboard formats and UTF-16 storage must not leak into DOM, script or other platform-neutral crates.

The Windows clipboard is process-global and exclusive while open. Direct Win32 access also requires unsafe FFI, while `rarog-platform-windows` inherits the workspace `unsafe_code = "forbid"` policy.

## Decision

`rarog-platform-windows` provides `WindowsClipboardService` and advertises `PlatformService::Clipboard` through `WindowsPlatformHost`.

The first adapter supports only Windows `CF_UNICODETEXT`. Reads return `None` when Unicode text is not available. Successful reads are converted to Rarog-owned UTF-8 `ClipboardText` and checked against the service's `ClipboardLimits`. Writes revalidate the supplied `ClipboardText` against the receiving service's limits before touching the system clipboard.

The adapter uses pinned Windows-only `clipboard-win` 5.4.1 as the narrow native implementation dependency. Its safe API owns the actual Win32 clipboard open/close and UTF-16 memory operations, so `rarog-platform-windows` remains free of unsafe code and raw clipboard handles.

Failure normalization is deliberately small:

- failure to acquire the global clipboard after bounded retries becomes `ClipboardError::Busy`;
- malformed/unreadable Unicode clipboard contents become `ClipboardError::InvalidData`;
- write failures after acquiring the clipboard become `ClipboardError::BackendFailure`;
- oversized text remains `ClipboardError::TextLimitExceeded`;
- non-Windows targets remain `ClipboardError::UnsupportedTarget`.

No clipboard object or native lock is retained across calls. Each read/write operation owns its clipboard guard only for the duration of that operation.

## Consequences

DOM/script layers can use a stable object-safe clipboard service without depending on Win32 types or unsafe code. The initial surface is intentionally text-only; HTML, images, file lists, custom formats, clipboard monitoring and permission/user-activation policy remain outside this adapter and can be layered later without changing the native ownership boundary.
