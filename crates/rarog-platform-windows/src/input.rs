use rarog_platform::{
    KeyState, KeyValue, KeyboardInputEvent, ModifierState, PhysicalKey, PlatformInputError,
    PlatformInputEvent, PlatformInputService, PlatformPoint, PointerAction, PointerButton,
    PointerButtons, PointerInputEvent, PointerKind, TextInputEvent, WheelDeltaMode, WheelInputEvent,
};
use std::collections::VecDeque;
use std::sync::Mutex;

const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_CHAR: u32 = 0x0102;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_SYSCHAR: u32 = 0x0106;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_MBUTTONDOWN: u32 = 0x0207;
const WM_MBUTTONUP: u32 = 0x0208;
const WM_XBUTTONDOWN: u32 = 0x020b;
const WM_XBUTTONUP: u32 = 0x020c;
const WM_MOUSELEAVE: u32 = 0x02a3;

const MK_LBUTTON: u16 = 0x0001;
const MK_RBUTTON: u16 = 0x0002;
const MK_SHIFT: u16 = 0x0004;
const MK_CONTROL: u16 = 0x0008;
const MK_MBUTTON: u16 = 0x0010;
const MK_XBUTTON1: u16 = 0x0020;
const MK_XBUTTON2: u16 = 0x0040;

const VK_BACK: usize = 0x08;
const VK_TAB: usize = 0x09;
const VK_RETURN: usize = 0x0d;
const VK_SHIFT: usize = 0x10;
const VK_CONTROL: usize = 0x11;
const VK_MENU: usize = 0x12;
const VK_PAUSE: usize = 0x13;
const VK_CAPITAL: usize = 0x14;
const VK_ESCAPE: usize = 0x1b;
const VK_SPACE: usize = 0x20;
const VK_PRIOR: usize = 0x21;
const VK_NEXT: usize = 0x22;
const VK_END: usize = 0x23;
const VK_HOME: usize = 0x24;
const VK_LEFT: usize = 0x25;
const VK_UP: usize = 0x26;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;
const VK_INSERT: usize = 0x2d;
const VK_DELETE: usize = 0x2e;
const VK_LWIN: usize = 0x5b;
const VK_RWIN: usize = 0x5c;
const VK_APPS: usize = 0x5d;
const VK_NUMLOCK: usize = 0x90;
const VK_SCROLL: usize = 0x91;
const VK_LSHIFT: usize = 0xa0;
const VK_RSHIFT: usize = 0xa1;
const VK_LCONTROL: usize = 0xa2;
const VK_RCONTROL: usize = 0xa3;
const VK_LMENU: usize = 0xa4;
const VK_RMENU: usize = 0xa5;

#[derive(Debug)]
pub struct WindowsInputService {
    state: Mutex<WindowsInputState>,
}

impl WindowsInputService {
    pub fn try_new() -> Result<Self, crate::WindowsPlatformError> {
        if cfg!(target_os = "windows") {
            Ok(Self {
                state: Mutex::new(WindowsInputState::default()),
            })
        } else {
            Err(crate::WindowsPlatformError::UnsupportedTarget)
        }
    }

    pub fn push_window_message(
        &self,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> Result<bool, PlatformInputError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PlatformInputError::BackendFailure)?;
        state.push_window_message(message, wparam, lparam)
    }

    pub fn push_wheel(
        &self,
        delta_x: f32,
        delta_y: f32,
        client_position: PlatformPoint,
    ) -> Result<(), PlatformInputError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PlatformInputError::BackendFailure)?;
        state.push_wheel(delta_x, delta_y, client_position)
    }

    pub fn set_modifier_state(
        &self,
        modifiers: ModifierState,
    ) -> Result<(), PlatformInputError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PlatformInputError::BackendFailure)?;
        state.modifiers = modifiers;
        Ok(())
    }
}

impl PlatformInputService for WindowsInputService {
    fn poll_event(&self) -> Result<Option<PlatformInputEvent>, PlatformInputError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PlatformInputError::BackendFailure)?;
        Ok(state.queue.pop_front())
    }
}

#[derive(Debug, Default)]
struct WindowsInputState {
    queue: VecDeque<PlatformInputEvent>,
    modifiers: ModifierState,
    pending_high_surrogate: Option<u16>,
}

impl WindowsInputState {
    fn push_window_message(
        &mut self,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> Result<bool, PlatformInputError> {
        match message {
            WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
                self.push_keyboard(message, wparam, lparam);
                Ok(true)
            }
            WM_CHAR | WM_SYSCHAR => {
                self.push_utf16_unit(wparam as u16);
                Ok(true)
            }
            WM_MOUSEMOVE
            | WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_RBUTTONDOWN
            | WM_RBUTTONUP
            | WM_MBUTTONDOWN
            | WM_MBUTTONUP
            | WM_XBUTTONDOWN
            | WM_XBUTTONUP
            | WM_MOUSELEAVE => {
                self.push_mouse(message, wparam, lparam)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn push_keyboard(&mut self, message: u32, virtual_key: usize, lparam: isize) {
        let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        let repeat = pressed && ((lparam as usize >> 30) & 1) != 0;
        self.update_modifier(virtual_key, pressed, repeat);
        let scan_code = ((lparam as usize >> 16) & 0xff) as u8;
        let extended = ((lparam as usize >> 24) & 1) != 0;
        self.queue
            .push_back(PlatformInputEvent::Keyboard(KeyboardInputEvent {
                state: if pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                },
                physical_key: physical_key(scan_code, extended),
                key: logical_key(virtual_key),
                repeat,
                modifiers: self.modifiers,
            }));
    }

    fn update_modifier(&mut self, virtual_key: usize, pressed: bool, repeat: bool) {
        match virtual_key {
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => self.modifiers.shift = pressed,
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => self.modifiers.control = pressed,
            VK_MENU | VK_LMENU | VK_RMENU => self.modifiers.alt = pressed,
            VK_LWIN | VK_RWIN => self.modifiers.meta = pressed,
            VK_CAPITAL if pressed && !repeat => self.modifiers.caps_lock = !self.modifiers.caps_lock,
            VK_NUMLOCK if pressed && !repeat => self.modifiers.num_lock = !self.modifiers.num_lock,
            _ => {}
        }
    }

    fn push_mouse(
        &mut self,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> Result<(), PlatformInputError> {
        let low_word = wparam as u16;
        let position = if message == WM_MOUSELEAVE {
            PlatformPoint::default()
        } else {
            mouse_position(lparam).validate()?
        };
        let mut modifiers = self.modifiers;
        modifiers.shift |= low_word & MK_SHIFT != 0;
        modifiers.control |= low_word & MK_CONTROL != 0;
        let buttons = mouse_buttons(low_word);
        let (action, button) = match message {
            WM_MOUSEMOVE => (PointerAction::Move, None),
            WM_MOUSELEAVE => (PointerAction::Leave, None),
            WM_LBUTTONDOWN => (PointerAction::Down, Some(PointerButton::Primary)),
            WM_LBUTTONUP => (PointerAction::Up, Some(PointerButton::Primary)),
            WM_RBUTTONDOWN => (PointerAction::Down, Some(PointerButton::Secondary)),
            WM_RBUTTONUP => (PointerAction::Up, Some(PointerButton::Secondary)),
            WM_MBUTTONDOWN => (PointerAction::Down, Some(PointerButton::Auxiliary)),
            WM_MBUTTONUP => (PointerAction::Up, Some(PointerButton::Auxiliary)),
            WM_XBUTTONDOWN | WM_XBUTTONUP => {
                let xbutton = ((wparam >> 16) & 0xffff) as u16;
                let button = match xbutton {
                    1 => PointerButton::Back,
                    2 => PointerButton::Forward,
                    value => PointerButton::Other(value),
                };
                let action = if message == WM_XBUTTONDOWN {
                    PointerAction::Down
                } else {
                    PointerAction::Up
                };
                (action, Some(button))
            }
            _ => return Ok(()),
        };
        let event = PointerInputEvent {
            action,
            kind: PointerKind::Mouse,
            pointer_id: 1,
            position,
            button,
            buttons,
            pressure: None,
            modifiers,
        };
        event.validate()?;
        self.queue.push_back(PlatformInputEvent::Pointer(event));
        Ok(())
    }

    fn push_wheel(
        &mut self,
        delta_x: f32,
        delta_y: f32,
        client_position: PlatformPoint,
    ) -> Result<(), PlatformInputError> {
        let event = WheelInputEvent {
            delta_x,
            delta_y,
            mode: WheelDeltaMode::Pixel,
            position: client_position,
            modifiers: self.modifiers,
        };
        event.validate()?;
        self.queue.push_back(PlatformInputEvent::Wheel(event));
        Ok(())
    }

    fn push_utf16_unit(&mut self, unit: u16) {
        if (0xd800..=0xdbff).contains(&unit) {
            if self.pending_high_surrogate.replace(unit).is_some() {
                self.push_character('\u{fffd}');
            }
            return;
        }
        if (0xdc00..=0xdfff).contains(&unit) {
            let Some(high) = self.pending_high_surrogate.take() else {
                self.push_character('\u{fffd}');
                return;
            };
            let high = u32::from(high - 0xd800);
            let low = u32::from(unit - 0xdc00);
            let scalar = 0x10000 + ((high << 10) | low);
            self.push_character(char::from_u32(scalar).unwrap_or('\u{fffd}'));
            return;
        }
        if self.pending_high_surrogate.take().is_some() {
            self.push_character('\u{fffd}');
        }
        self.push_character(char::from_u32(u32::from(unit)).unwrap_or('\u{fffd}'));
    }

    fn push_character(&mut self, character: char) {
        self.queue
            .push_back(PlatformInputEvent::Text(TextInputEvent::Commit {
                text: character.to_string(),
            }));
    }
}

fn mouse_position(lparam: isize) -> PlatformPoint {
    let packed = lparam as u32;
    let x = (packed as u16 as i16) as f32;
    let y = ((packed >> 16) as u16 as i16) as f32;
    PlatformPoint { x, y }
}

fn mouse_buttons(word: u16) -> PointerButtons {
    let mut bits = 0;
    if word & MK_LBUTTON != 0 {
        bits |= PointerButtons::PRIMARY;
    }
    if word & MK_RBUTTON != 0 {
        bits |= PointerButtons::SECONDARY;
    }
    if word & MK_MBUTTON != 0 {
        bits |= PointerButtons::AUXILIARY;
    }
    if word & MK_XBUTTON1 != 0 {
        bits |= PointerButtons::BACK;
    }
    if word & MK_XBUTTON2 != 0 {
        bits |= PointerButtons::FORWARD;
    }
    PointerButtons::from_bits(bits)
}

fn physical_key(scan_code: u8, extended: bool) -> PhysicalKey {
    let code = match (scan_code, extended) {
        (0x01, _) => "Escape",
        (0x02, _) => "Digit1",
        (0x03, _) => "Digit2",
        (0x04, _) => "Digit3",
        (0x05, _) => "Digit4",
        (0x06, _) => "Digit5",
        (0x07, _) => "Digit6",
        (0x08, _) => "Digit7",
        (0x09, _) => "Digit8",
        (0x0a, _) => "Digit9",
        (0x0b, _) => "Digit0",
        (0x0c, _) => "Minus",
        (0x0d, _) => "Equal",
        (0x0e, _) => "Backspace",
        (0x0f, _) => "Tab",
        (0x10, _) => "KeyQ",
        (0x11, _) => "KeyW",
        (0x12, _) => "KeyE",
        (0x13, _) => "KeyR",
        (0x14, _) => "KeyT",
        (0x15, _) => "KeyY",
        (0x16, _) => "KeyU",
        (0x17, _) => "KeyI",
        (0x18, _) => "KeyO",
        (0x19, _) => "KeyP",
        (0x1a, _) => "BracketLeft",
        (0x1b, _) => "BracketRight",
        (0x1c, true) => "NumpadEnter",
        (0x1c, false) => "Enter",
        (0x1d, true) => "ControlRight",
        (0x1d, false) => "ControlLeft",
        (0x1e, _) => "KeyA",
        (0x1f, _) => "KeyS",
        (0x20, _) => "KeyD",
        (0x21, _) => "KeyF",
        (0x22, _) => "KeyG",
        (0x23, _) => "KeyH",
        (0x24, _) => "KeyJ",
        (0x25, _) => "KeyK",
        (0x26, _) => "KeyL",
        (0x27, _) => "Semicolon",
        (0x28, _) => "Quote",
        (0x29, _) => "Backquote",
        (0x2a, _) => "ShiftLeft",
        (0x2b, _) => "Backslash",
        (0x2c, _) => "KeyZ",
        (0x2d, _) => "KeyX",
        (0x2e, _) => "KeyC",
        (0x2f, _) => "KeyV",
        (0x30, _) => "KeyB",
        (0x31, _) => "KeyN",
        (0x32, _) => "KeyM",
        (0x33, _) => "Comma",
        (0x34, _) => "Period",
        (0x35, true) => "NumpadDivide",
        (0x35, false) => "Slash",
        (0x36, _) => "ShiftRight",
        (0x37, _) => "NumpadMultiply",
        (0x38, true) => "AltRight",
        (0x38, false) => "AltLeft",
        (0x39, _) => "Space",
        (0x3a, _) => "CapsLock",
        (0x3b, _) => "F1",
        (0x3c, _) => "F2",
        (0x3d, _) => "F3",
        (0x3e, _) => "F4",
        (0x3f, _) => "F5",
        (0x40, _) => "F6",
        (0x41, _) => "F7",
        (0x42, _) => "F8",
        (0x43, _) => "F9",
        (0x44, _) => "F10",
        (0x45, _) => "NumLock",
        (0x46, _) => "ScrollLock",
        (0x47, true) => "Home",
        (0x47, false) => "Numpad7",
        (0x48, true) => "ArrowUp",
        (0x48, false) => "Numpad8",
        (0x49, true) => "PageUp",
        (0x49, false) => "Numpad9",
        (0x4a, _) => "NumpadSubtract",
        (0x4b, true) => "ArrowLeft",
        (0x4b, false) => "Numpad4",
        (0x4c, _) => "Numpad5",
        (0x4d, true) => "ArrowRight",
        (0x4d, false) => "Numpad6",
        (0x4e, _) => "NumpadAdd",
        (0x4f, true) => "End",
        (0x4f, false) => "Numpad1",
        (0x50, true) => "ArrowDown",
        (0x50, false) => "Numpad2",
        (0x51, true) => "PageDown",
        (0x51, false) => "Numpad3",
        (0x52, true) => "Insert",
        (0x52, false) => "Numpad0",
        (0x53, true) => "Delete",
        (0x53, false) => "NumpadDecimal",
        (0x57, _) => "F11",
        (0x58, _) => "F12",
        (0x5b, _) => "MetaLeft",
        (0x5c, _) => "MetaRight",
        (0x5d, _) => "ContextMenu",
        _ => return PhysicalKey::Unidentified,
    };
    PhysicalKey::Code(code.to_owned())
}

fn logical_key(virtual_key: usize) -> KeyValue {
    let named = match virtual_key {
        VK_BACK => "Backspace",
        VK_TAB => "Tab",
        VK_RETURN => "Enter",
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => "Shift",
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => "Control",
        VK_MENU | VK_LMENU | VK_RMENU => "Alt",
        VK_PAUSE => "Pause",
        VK_CAPITAL => "CapsLock",
        VK_ESCAPE => "Escape",
        VK_PRIOR => "PageUp",
        VK_NEXT => "PageDown",
        VK_END => "End",
        VK_HOME => "Home",
        VK_LEFT => "ArrowLeft",
        VK_UP => "ArrowUp",
        VK_RIGHT => "ArrowRight",
        VK_DOWN => "ArrowDown",
        VK_INSERT => "Insert",
        VK_DELETE => "Delete",
        VK_LWIN | VK_RWIN => "Meta",
        VK_APPS => "ContextMenu",
        VK_NUMLOCK => "NumLock",
        VK_SCROLL => "ScrollLock",
        0x70..=0x87 => return KeyValue::Named(format!("F{}", virtual_key - 0x6f)),
        VK_SPACE => return KeyValue::Character(" ".to_owned()),
        _ => return KeyValue::Unidentified,
    };
    KeyValue::Named(named.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_lparam(scan_code: u8, extended: bool, repeat: bool) -> isize {
        ((usize::from(scan_code) << 16)
            | ((extended as usize) << 24)
            | ((repeat as usize) << 30)) as isize
    }

    fn pop(state: &mut WindowsInputState) -> PlatformInputEvent {
        state.queue.pop_front().expect("expected normalized event")
    }

    #[test]
    fn keyboard_messages_preserve_physical_key_and_modifier_state() {
        let mut state = WindowsInputState::default();
        state
            .push_window_message(WM_KEYDOWN, VK_SHIFT, key_lparam(0x2a, false, false))
            .unwrap();
        match pop(&mut state) {
            PlatformInputEvent::Keyboard(event) => {
                assert_eq!(event.physical_key, PhysicalKey::Code("ShiftLeft".into()));
                assert!(event.modifiers.shift);
            }
            event => panic!("unexpected event: {event:?}"),
        }

        state
            .push_window_message(WM_KEYDOWN, usize::from(b'A'), key_lparam(0x1e, false, false))
            .unwrap();
        match pop(&mut state) {
            PlatformInputEvent::Keyboard(event) => {
                assert_eq!(event.physical_key, PhysicalKey::Code("KeyA".into()));
                assert_eq!(event.key, KeyValue::Unidentified);
                assert!(event.modifiers.shift);
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn key_repeat_and_extended_navigation_are_preserved() {
        let mut state = WindowsInputState::default();
        state
            .push_window_message(WM_KEYDOWN, VK_LEFT, key_lparam(0x4b, true, true))
            .unwrap();
        match pop(&mut state) {
            PlatformInputEvent::Keyboard(event) => {
                assert!(event.repeat);
                assert_eq!(event.physical_key, PhysicalKey::Code("ArrowLeft".into()));
                assert_eq!(event.key, KeyValue::Named("ArrowLeft".into()));
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn mouse_messages_keep_signed_client_coordinates_and_buttons() {
        let mut state = WindowsInputState::default();
        let lparam = ((20_u32 << 16) | u32::from((-10_i16) as u16)) as isize;
        state
            .push_window_message(WM_LBUTTONDOWN, usize::from(MK_LBUTTON | MK_SHIFT), lparam)
            .unwrap();
        match pop(&mut state) {
            PlatformInputEvent::Pointer(event) => {
                assert_eq!(event.action, PointerAction::Down);
                assert_eq!(event.button, Some(PointerButton::Primary));
                assert_eq!(event.position, PlatformPoint { x: -10.0, y: 20.0 });
                assert!(event.buttons.contains(PointerButtons::PRIMARY));
                assert!(event.modifiers.shift);
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn utf16_character_messages_emit_committed_unicode_text() {
        let mut state = WindowsInputState::default();
        state.push_window_message(WM_CHAR, 0xd83d, 0).unwrap();
        assert!(state.queue.is_empty());
        state.push_window_message(WM_CHAR, 0xde00, 0).unwrap();
        assert_eq!(
            pop(&mut state),
            PlatformInputEvent::Text(TextInputEvent::Commit { text: "😀".into() })
        );
    }

    #[test]
    fn malformed_surrogates_use_unicode_replacement_character() {
        let mut state = WindowsInputState::default();
        state.push_window_message(WM_CHAR, 0xd83d, 0).unwrap();
        state.push_window_message(WM_CHAR, usize::from(b'A'), 0).unwrap();
        assert_eq!(
            pop(&mut state),
            PlatformInputEvent::Text(TextInputEvent::Commit {
                text: "�".into()
            })
        );
        assert_eq!(
            pop(&mut state),
            PlatformInputEvent::Text(TextInputEvent::Commit { text: "A".into() })
        );
    }

    #[test]
    fn wheel_bridge_requires_client_space_finite_geometry() {
        let mut state = WindowsInputState::default();
        state
            .push_wheel(0.0, -120.0, PlatformPoint { x: 5.0, y: 7.0 })
            .unwrap();
        match pop(&mut state) {
            PlatformInputEvent::Wheel(event) => {
                assert_eq!(event.position, PlatformPoint { x: 5.0, y: 7.0 });
                assert_eq!(event.delta_y, -120.0);
            }
            event => panic!("unexpected event: {event:?}"),
        }
        assert_eq!(
            state.push_wheel(f32::NAN, 0.0, PlatformPoint::default()),
            Err(PlatformInputError::InvalidWheelDelta)
        );
    }
}
