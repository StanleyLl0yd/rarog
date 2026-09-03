# ADR-0061: Windows IME and text-input adapter

Status: accepted

## Context

ADR-0059 defines a platform-neutral text-input state and composition event model, while ADR-0060 keeps Win32 keyboard message normalization inside `rarog-platform-windows`. Windows IME integration additionally requires the engine to expose whether text input is enabled and where the active caret is located, while the native window layer receives composition/result strings through IMM32 or a future text-services backend.

Calling native IME APIs directly from the cross-platform contract would leak HWND/HIMC lifetimes and require FFI inside the portable boundary. Treating IME strings as ordinary `WM_CHAR` input would also lose composition lifecycle and selection information.

## Decision

`WindowsInputService` implements both `PlatformInputService` and `PlatformTextInputService`. Text-input state is validated and retained so the native Windows window layer can query the current enabled flag and caret rectangle when positioning an IME candidate/composition UI.

The Windows-specific bridge accepts composition and committed text as UTF-16 code units. This matches the representation returned by Windows APIs without exposing native handles or buffers to `rarog-platform`. Composition start, update, end and commit operations enqueue `TextInputEvent` values into the same input queue used by keyboard and pointer input, preserving a single deterministic event order.

Optional composition selection offsets arrive as UTF-16 offsets and are converted to Unicode scalar indices before crossing into the neutral event model. Out-of-bounds ranges, reversed ranges and boundaries that split a valid surrogate pair are rejected as `PlatformInputError::InvalidTextRange`. Text decoding uses replacement characters for malformed UTF-16 rather than propagating invalid Unicode.

`WindowsPlatformHost` now advertises `InputIme` and exposes the same input object through both neutral input service interfaces. Clipboard remains a separate capability.

The layer that owns the HWND/HIMC remains responsible for obtaining IMM32 composition/result buffers, applying the retained caret rectangle to native IME positioning and forwarding those buffers into this adapter. This keeps all native handles and any required unsafe FFI outside the Rarog-owned cross-platform contracts.

## Consequences

IME composition and ordinary keyboard/character events share one ordered queue. Rarog receives Unicode plus Rarog-owned selection ranges, while the native host retains control over Windows API lifetimes. The design can later be connected to IMM32 or TSF without changing DOM/script-facing input types.

This slice does not implement DOM `beforeinput`/`input` event synthesis, editing commands, selection ownership, text controls, contenteditable behavior or a concrete IMM32/TSF FFI layer.
