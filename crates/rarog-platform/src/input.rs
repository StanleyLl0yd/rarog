use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModifierState {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalKey {
    Code(String),
    Unidentified,
}

impl PhysicalKey {
    pub fn code(value: impl Into<String>) -> Result<Self, PlatformInputError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(PlatformInputError::EmptyKeyCode);
        }
        Ok(Self::Code(value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyValue {
    Character(String),
    Named(String),
    Dead,
    Unidentified,
}

impl KeyValue {
    pub fn character(value: impl Into<String>) -> Result<Self, PlatformInputError> {
        let value = value.into();
        if value.is_empty() {
            return Err(PlatformInputError::EmptyKeyValue);
        }
        Ok(Self::Character(value))
    }

    pub fn named(value: impl Into<String>) -> Result<Self, PlatformInputError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(PlatformInputError::EmptyKeyValue);
        }
        Ok(Self::Named(value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardInputEvent {
    pub state: KeyState,
    pub physical_key: PhysicalKey,
    pub key: KeyValue,
    pub repeat: bool,
    pub modifiers: ModifierState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlatformPoint {
    pub x: f32,
    pub y: f32,
}

impl PlatformPoint {
    pub fn validate(self) -> Result<Self, PlatformInputError> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(PlatformInputError::InvalidCoordinate);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlatformRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PlatformRect {
    pub fn validate(self) -> Result<Self, PlatformInputError> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.width < 0.0
            || self.height < 0.0
        {
            return Err(PlatformInputError::InvalidRectangle);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Mouse,
    Touch,
    Pen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerAction {
    Move,
    Down,
    Up,
    Enter,
    Leave,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Auxiliary,
    Secondary,
    Back,
    Forward,
    Other(u16),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerButtons(u16);

impl PointerButtons {
    pub const PRIMARY: u16 = 1 << 0;
    pub const SECONDARY: u16 = 1 << 1;
    pub const AUXILIARY: u16 = 1 << 2;
    pub const BACK: u16 = 1 << 3;
    pub const FORWARD: u16 = 1 << 4;

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, bits: u16) -> bool {
        self.0 & bits == bits
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerInputEvent {
    pub action: PointerAction,
    pub kind: PointerKind,
    pub pointer_id: u64,
    pub position: PlatformPoint,
    pub button: Option<PointerButton>,
    pub buttons: PointerButtons,
    pub pressure: Option<f32>,
    pub modifiers: ModifierState,
}

impl PointerInputEvent {
    pub fn validate(&self) -> Result<(), PlatformInputError> {
        self.position.validate()?;
        if let Some(pressure) = self.pressure {
            if !pressure.is_finite() || !(0.0..=1.0).contains(&pressure) {
                return Err(PlatformInputError::InvalidPressure);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelDeltaMode {
    Pixel,
    Line,
    Page,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WheelInputEvent {
    pub delta_x: f32,
    pub delta_y: f32,
    pub mode: WheelDeltaMode,
    pub position: PlatformPoint,
    pub modifiers: ModifierState,
}

impl WheelInputEvent {
    pub fn validate(&self) -> Result<(), PlatformInputError> {
        self.position.validate()?;
        if !self.delta_x.is_finite() || !self.delta_y.is_finite() {
            return Err(PlatformInputError::InvalidWheelDelta);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn try_new(start: usize, end: usize) -> Result<Self, PlatformInputError> {
        if start > end {
            return Err(PlatformInputError::InvalidTextRange);
        }
        Ok(Self { start, end })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextInputEvent {
    CompositionStart,
    CompositionUpdate {
        text: String,
        selection: Option<TextRange>,
    },
    CompositionEnd {
        text: String,
    },
    Commit {
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformInputEvent {
    Keyboard(KeyboardInputEvent),
    Pointer(PointerInputEvent),
    Wheel(WheelInputEvent),
    Text(TextInputEvent),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextInputState {
    pub enabled: bool,
    pub caret_rect: PlatformRect,
}

impl TextInputState {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            caret_rect: PlatformRect::default(),
        }
    }

    pub fn validate(&self) -> Result<(), PlatformInputError> {
        self.caret_rect.validate().map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformInputError {
    UnsupportedTarget,
    EmptyKeyCode,
    EmptyKeyValue,
    InvalidCoordinate,
    InvalidRectangle,
    InvalidPressure,
    InvalidWheelDelta,
    InvalidTextRange,
    BackendFailure,
}

impl fmt::Display for PlatformInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedTarget => "platform input is unavailable on this target",
            Self::EmptyKeyCode => "physical key code must not be empty",
            Self::EmptyKeyValue => "key value must not be empty",
            Self::InvalidCoordinate => "input coordinates must be finite",
            Self::InvalidRectangle => "input rectangle must be finite and non-negative",
            Self::InvalidPressure => "pointer pressure must be finite and within 0..=1",
            Self::InvalidWheelDelta => "wheel deltas must be finite",
            Self::InvalidTextRange => "text range start must not exceed end",
            Self::BackendFailure => "platform input backend failed",
        })
    }
}

impl std::error::Error for PlatformInputError {}

pub trait PlatformInputService: Send + Sync {
    fn poll_event(&self) -> Result<Option<PlatformInputEvent>, PlatformInputError>;
}

pub trait PlatformTextInputService: Send + Sync {
    fn set_text_input_state(&self, state: &TextInputState) -> Result<(), PlatformInputError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_values_and_codes_reject_empty_inputs() {
        assert_eq!(
            PhysicalKey::code("   "),
            Err(PlatformInputError::EmptyKeyCode)
        );
        assert_eq!(
            KeyValue::character(""),
            Err(PlatformInputError::EmptyKeyValue)
        );
        assert_eq!(
            KeyValue::named(" "),
            Err(PlatformInputError::EmptyKeyValue)
        );
    }

    #[test]
    fn pointer_and_wheel_geometry_is_validated() {
        let pointer = PointerInputEvent {
            action: PointerAction::Move,
            kind: PointerKind::Mouse,
            pointer_id: 1,
            position: PlatformPoint { x: 10.0, y: 20.0 },
            button: None,
            buttons: PointerButtons::default(),
            pressure: Some(0.5),
            modifiers: ModifierState::default(),
        };
        assert_eq!(pointer.validate(), Ok(()));
        let invalid = PointerInputEvent {
            pressure: Some(2.0),
            ..pointer
        };
        assert_eq!(invalid.validate(), Err(PlatformInputError::InvalidPressure));

        let wheel = WheelInputEvent {
            delta_x: 0.0,
            delta_y: f32::NAN,
            mode: WheelDeltaMode::Pixel,
            position: PlatformPoint::default(),
            modifiers: ModifierState::default(),
        };
        assert_eq!(wheel.validate(), Err(PlatformInputError::InvalidWheelDelta));
    }

    #[test]
    fn text_ranges_and_input_state_are_explicitly_validated() {
        assert_eq!(TextRange::try_new(2, 4).unwrap(), TextRange { start: 2, end: 4 });
        assert_eq!(
            TextRange::try_new(4, 2),
            Err(PlatformInputError::InvalidTextRange)
        );
        let state = TextInputState {
            enabled: true,
            caret_rect: PlatformRect {
                x: 10.0,
                y: 20.0,
                width: 1.0,
                height: 18.0,
            },
        };
        assert_eq!(state.validate(), Ok(()));
    }

    #[test]
    fn service_traits_are_object_safe() {
        fn accepts_input(_: &dyn PlatformInputService) {}
        fn accepts_text(_: &dyn PlatformTextInputService) {}

        struct Fixture;
        impl PlatformInputService for Fixture {
            fn poll_event(&self) -> Result<Option<PlatformInputEvent>, PlatformInputError> {
                Ok(None)
            }
        }
        impl PlatformTextInputService for Fixture {
            fn set_text_input_state(
                &self,
                _state: &TextInputState,
            ) -> Result<(), PlatformInputError> {
                Ok(())
            }
        }

        let fixture = Fixture;
        accepts_input(&fixture);
        accepts_text(&fixture);
    }
}
