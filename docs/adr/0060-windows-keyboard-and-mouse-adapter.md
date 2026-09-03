# ADR-0060: Windows keyboard and mouse adapter

Status: accepted

## Context

ADR-0059 defines platform-neutral keyboard, pointer, wheel and text-commit events. R2 now needs a Windows adapter without leaking Win32 message identifiers, virtual-key values, scan-code packing or window handles into `rarog-platform`.

Windows keyboard input has two different useful representations. Key messages carry physical scan codes and stable named-key information, while character messages are produced after the active keyboard layout, dead-key and system translation rules have been applied. Guessing printable logical characters directly from virtual-key values would therefore be incorrect for non-US layouts.

Mouse messages carry signed client coordinates and button/modifier state in packed integer fields. Wheel message coordinates are screen-relative, so blindly treating them as client coordinates would also be incorrect without a window handle conversion.

## Decision

`rarog-platform-windows` owns `WindowsInputService`. The embedding Windows window procedure forwards relevant numeric message fields through `push_window_message`; the adapter decodes them into Rarog-owned events queued behind `PlatformInputService`.

Key down/up messages normalize common Set-1 scan codes into Web-compatible physical `code` strings, preserve repeat state, maintain modifier/toggle state and expose only stable named logical keys. Printable logical characters are deliberately left unidentified at key-message time. `WM_CHAR` and `WM_SYSCHAR` are instead decoded as UTF-16 and emitted as committed text, including surrogate-pair handling and replacement characters for malformed sequences.

Mouse move/button/leave messages normalize signed client coordinates, button identity, button bitsets and modifier state. A separate `push_wheel` bridge accepts already-normalized client coordinates and finite deltas; native screen-to-client conversion remains the responsibility of the Windows window layer that owns the HWND.

The adapter uses no unsafe code and adds no Win32 dependency in this slice. Win32 numeric constants and bit packing remain private to `rarog-platform-windows`; no native handle or message representation appears in `rarog-platform`.

`WindowsPlatformHost` advertises generic input only when constructed on Windows and exposes `WindowsInputService` through the neutral `PlatformInputService`. IME/text-input and clipboard capabilities remain disabled until their dedicated adapters are implemented.

## Consequences

Keyboard layouts are not hard-coded into Rarog, physical keyboard identity stays independent of generated text, and the engine receives deterministic cross-platform input events. The window/event-loop layer remains responsible for feeding messages and for any native coordinate transformation that requires an HWND.

This slice does not register a native window class, own a Win32 message pump, implement raw input, pointer/touch APIs, gestures, IME composition or clipboard access. Those responsibilities remain separate host-adapter slices.
