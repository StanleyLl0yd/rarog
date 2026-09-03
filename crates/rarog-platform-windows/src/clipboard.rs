use rarog_platform::{
    ClipboardError, ClipboardLimits, ClipboardText, PlatformClipboardService,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsClipboardService {
    limits: ClipboardLimits,
}

impl WindowsClipboardService {
    pub fn try_new(limits: ClipboardLimits) -> Result<Self, ClipboardError> {
        if !limits.is_valid() {
            return Err(ClipboardError::InvalidLimits);
        }
        if !cfg!(target_os = "windows") {
            return Err(ClipboardError::UnsupportedTarget);
        }
        Ok(Self { limits })
    }

    pub fn with_default_limits() -> Result<Self, ClipboardError> {
        Self::try_new(ClipboardLimits::default())
    }
}

impl PlatformClipboardService for WindowsClipboardService {
    fn limits(&self) -> ClipboardLimits {
        self.limits
    }

    fn read_text(&self) -> Result<Option<ClipboardText>, ClipboardError> {
        read_system_text(self.limits)
    }

    fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardError> {
        let validated = ClipboardText::try_new(text.as_str(), self.limits)?;
        write_system_text(&validated)
    }
}

#[cfg(target_os = "windows")]
fn read_system_text(limits: ClipboardLimits) -> Result<Option<ClipboardText>, ClipboardError> {
    use clipboard_win::{Clipboard, Format, Getter, formats::Unicode};

    let _clipboard = Clipboard::new_attempts(5).map_err(|_| ClipboardError::Busy)?;
    if !Unicode.is_format_avail() {
        return Ok(None);
    }

    let mut text = String::new();
    Unicode
        .read_clipboard(&mut text)
        .map_err(|_| ClipboardError::InvalidData)?;
    ClipboardText::try_new(text, limits).map(Some)
}

#[cfg(not(target_os = "windows"))]
fn read_system_text(_limits: ClipboardLimits) -> Result<Option<ClipboardText>, ClipboardError> {
    Err(ClipboardError::UnsupportedTarget)
}

#[cfg(target_os = "windows")]
fn write_system_text(text: &ClipboardText) -> Result<(), ClipboardError> {
    use clipboard_win::{Clipboard, Setter, formats::Unicode};

    let _clipboard = Clipboard::new_attempts(5).map_err(|_| ClipboardError::Busy)?;
    let value = text.as_str();
    Unicode
        .write_clipboard(&value)
        .map_err(|_| ClipboardError::BackendFailure)
}

#[cfg(not(target_os = "windows"))]
fn write_system_text(_text: &ClipboardText) -> Result<(), ClipboardError> {
    Err(ClipboardError::UnsupportedTarget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_validates_limits_before_target_availability() {
        assert_eq!(
            WindowsClipboardService::try_new(ClipboardLimits { max_text_bytes: 0 }),
            Err(ClipboardError::InvalidLimits)
        );
    }

    #[test]
    fn construction_matches_compilation_target() {
        let result = WindowsClipboardService::with_default_limits();
        if cfg!(target_os = "windows") {
            let service = result.expect("Windows target should expose clipboard service");
            assert_eq!(service.limits(), ClipboardLimits::default());
        } else {
            assert_eq!(result, Err(ClipboardError::UnsupportedTarget));
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn service_rejects_text_created_with_a_larger_external_limit() {
        let service = WindowsClipboardService::try_new(ClipboardLimits { max_text_bytes: 4 })
            .expect("Windows target should expose clipboard service");
        let text = ClipboardText::try_new(
            "Rarog",
            ClipboardLimits {
                max_text_bytes: 1024,
            },
        )
        .unwrap();
        assert_eq!(
            service.write_text(&text),
            Err(ClipboardError::TextLimitExceeded { bytes: 5, limit: 4 })
        );
    }
}
