use std::fmt;

pub const DEFAULT_MAX_CLIPBOARD_TEXT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipboardLimits {
    pub max_text_bytes: usize,
}

impl ClipboardLimits {
    pub const fn is_valid(self) -> bool {
        self.max_text_bytes > 0
    }
}

impl Default for ClipboardLimits {
    fn default() -> Self {
        Self {
            max_text_bytes: DEFAULT_MAX_CLIPBOARD_TEXT_BYTES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardText {
    text: String,
}

impl ClipboardText {
    pub fn try_new(text: impl Into<String>, limits: ClipboardLimits) -> Result<Self, ClipboardError> {
        if !limits.is_valid() {
            return Err(ClipboardError::InvalidLimits);
        }
        let text = text.into();
        if text.len() > limits.max_text_bytes {
            return Err(ClipboardError::TextLimitExceeded {
                bytes: text.len(),
                limit: limits.max_text_bytes,
            });
        }
        Ok(Self { text })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn into_string(self) -> String {
        self.text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    UnsupportedTarget,
    InvalidLimits,
    TextLimitExceeded { bytes: usize, limit: usize },
    Busy,
    InvalidData,
    BackendFailure,
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => formatter.write_str("clipboard is unavailable on this target"),
            Self::InvalidLimits => formatter.write_str("clipboard limits must be non-zero"),
            Self::TextLimitExceeded { bytes, limit } => {
                write!(formatter, "clipboard text requires {bytes} bytes; limit is {limit}")
            }
            Self::Busy => formatter.write_str("clipboard is currently unavailable"),
            Self::InvalidData => formatter.write_str("clipboard data is invalid"),
            Self::BackendFailure => formatter.write_str("clipboard backend failed"),
        }
    }
}

impl std::error::Error for ClipboardError {}

pub trait PlatformClipboardService: Send + Sync {
    fn limits(&self) -> ClipboardLimits {
        ClipboardLimits::default()
    }

    fn read_text(&self) -> Result<Option<ClipboardText>, ClipboardError>;

    fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn clipboard_text_is_bounded_by_utf8_bytes() {
        let limits = ClipboardLimits { max_text_bytes: 4 };
        assert_eq!(ClipboardText::try_new("test", limits).unwrap().as_str(), "test");
        assert_eq!(
            ClipboardText::try_new("тест", limits),
            Err(ClipboardError::TextLimitExceeded { bytes: 8, limit: 4 })
        );
    }

    #[test]
    fn clipboard_service_is_object_safe() {
        struct Fixture(Mutex<Option<ClipboardText>>);
        impl PlatformClipboardService for Fixture {
            fn read_text(&self) -> Result<Option<ClipboardText>, ClipboardError> {
                Ok(self.0.lock().unwrap().clone())
            }

            fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardError> {
                *self.0.lock().unwrap() = Some(text.clone());
                Ok(())
            }
        }

        fn round_trip(service: &dyn PlatformClipboardService) {
            let text = ClipboardText::try_new("Rarog", service.limits()).unwrap();
            service.write_text(&text).unwrap();
            assert_eq!(service.read_text().unwrap().unwrap(), text);
        }

        round_trip(&Fixture(Mutex::new(None)));
    }
}
