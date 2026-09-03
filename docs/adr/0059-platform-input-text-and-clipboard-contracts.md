# ADR-0059: Platform-neutral input, text-input and clipboard contracts

Status: accepted

## Context

R2 needs keyboard, pointer, IME/text-input and clipboard support on Windows, but DOM/event/engine code must not depend on Win32 message identifiers, virtual-key constants, scan-code bit packing, HIMC handles, HWND values or clipboard handles.

Input and text composition also have different lifecycles. Hardware key/pointer events are polled from a platform event source, while IME integration additionally requires the engine to publish text-input state such as whether composition is enabled and where the caret is located. Clipboard access is a separate capability with its own data-size and availability failures.

## Decision

`rarog-platform` owns the normalized contracts. `rarog-platform-windows` translates Win32 state into these types and never exports native handles or constants through the public cross-platform API.

The neutral input model includes modifier state, physical-key identity, logical key values, key press/release state, mouse/touch/pen pointer actions, button state, finite pointer coordinates and pressure, wheel deltas, composition lifecycle events, committed text and validated text ranges. `PlatformInputService` supplies normalized events; `PlatformTextInputService` receives validated text-input/caret state.

Physical key codes and named logical key values are owned strings rather than a prematurely closed enum. This allows the Windows adapter to normalize toward Web-compatible `code`/`key` vocabulary while later platforms can use the same boundary without changing the crate ABI every time a key is added.

The first clipboard contract deliberately exposes bounded UTF-8 plain text through `PlatformClipboardService`. The value is validated against `ClipboardLimits` before crossing into the platform adapter. Additional clipboard MIME formats may extend the Rarog-owned contract later; native clipboard format identifiers remain backend details.

`PlatformCapabilities` distinguishes generic input, IME/text input and clipboard availability. `PlatformHost` exposes optional object-safe services for each capability in the same style as the existing font service.

## Consequences

Engine and DOM layers can consume normalized events and clipboard/text services without conditional Win32 code. Windows keyboard/mouse, IME and clipboard implementations can be delivered independently behind the same host object.

This slice does not define DOM `KeyboardEvent`, `PointerEvent`, `InputEvent` or Clipboard API bindings. It also does not define focus routing, hit testing, accelerator policy, drag-and-drop, touch gesture recognition or permission prompts. Those consume these platform contracts at higher layers.
