use rarog_platform::{PlatformCapabilities, PlatformHost};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsPlatformError {
    UnsupportedTarget,
}

impl fmt::Display for WindowsPlatformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => {
                formatter.write_str("Windows platform host is unavailable on this target")
            }
        }
    }
}

impl std::error::Error for WindowsPlatformError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsPlatformHost {
    _private: (),
}

impl WindowsPlatformHost {
    pub fn try_new() -> Result<Self, WindowsPlatformError> {
        if cfg!(target_os = "windows") {
            Ok(Self { _private: () })
        } else {
            Err(WindowsPlatformError::UnsupportedTarget)
        }
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }
}

impl PlatformHost for WindowsPlatformHost {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_matches_compilation_target() {
        let result = WindowsPlatformHost::try_new();
        if cfg!(target_os = "windows") {
            let host = result.expect("Windows CI target should expose the Windows host boundary");
            assert_eq!(host.name(), "windows");
            assert_eq!(host.capabilities(), PlatformCapabilities::NONE);
        } else {
            assert_eq!(result, Err(WindowsPlatformError::UnsupportedTarget));
        }
    }
}
