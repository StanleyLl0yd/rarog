use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeErrorKind {
    UnsupportedTarget,
    ComUnavailable,
    ProviderUnavailable,
    UnsupportedPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsAccessibilityNativeError {
    kind: WindowsAccessibilityNativeErrorKind,
}

impl WindowsAccessibilityNativeError {
    const fn new(kind: WindowsAccessibilityNativeErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> WindowsAccessibilityNativeErrorKind {
        self.kind
    }
}

impl fmt::Display for WindowsAccessibilityNativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Windows accessibility native error: {:?}",
            self.kind
        )
    }
}

impl std::error::Error for WindowsAccessibilityNativeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeEventKind {
    FocusChanged,
    Invoked,
    StateChanged,
    NameChanged,
    ValueChanged,
    BoundsChanged,
    TreeChanged,
}

#[derive(Debug, Default)]
pub struct WindowsAccessibilityNativeBridge;

impl WindowsAccessibilityNativeBridge {
    pub fn try_new() -> Result<Self, WindowsAccessibilityNativeError> {
        try_probe_com()?;
        Ok(Self)
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub fn publish_event(
        &mut self,
        provider_serial: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        publish_win_event(provider_serial, kind)
    }
}

#[cfg(target_os = "windows")]
fn try_probe_com() -> Result<(), WindowsAccessibilityNativeError> {
    use std::ptr::null;
    use windows_sys::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

    const RPC_E_CHANGED_MODE: i32 = 0x8001_0106u32 as i32;
    let result = unsafe { CoInitializeEx(null(), COINIT_MULTITHREADED) };
    if result >= 0 {
        unsafe {
            CoUninitialize();
        }
        return Ok(());
    }
    if result == RPC_E_CHANGED_MODE {
        return Ok(());
    }
    Err(WindowsAccessibilityNativeError::new(
        WindowsAccessibilityNativeErrorKind::ComUnavailable,
    ))
}

#[cfg(not(target_os = "windows"))]
fn try_probe_com() -> Result<(), WindowsAccessibilityNativeError> {
    Err(WindowsAccessibilityNativeError::new(
        WindowsAccessibilityNativeErrorKind::UnsupportedTarget,
    ))
}

#[cfg(target_os = "windows")]
fn publish_win_event(
    provider_serial: u64,
    kind: WindowsAccessibilityNativeEventKind,
) -> Result<(), WindowsAccessibilityNativeError> {
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::Accessibility::NotifyWinEvent;

    const OBJID_CLIENT: i32 = -4;
    const EVENT_OBJECT_REORDER: u32 = 0x8004;
    const EVENT_OBJECT_FOCUS: u32 = 0x8005;
    const EVENT_OBJECT_STATECHANGE: u32 = 0x800A;
    const EVENT_OBJECT_LOCATIONCHANGE: u32 = 0x800B;
    const EVENT_OBJECT_NAMECHANGE: u32 = 0x800C;
    const EVENT_OBJECT_VALUECHANGE: u32 = 0x800E;
    const EVENT_OBJECT_INVOKED: u32 = 0x8013;

    if provider_serial == 0 || provider_serial > i32::MAX as u64 {
        return Err(WindowsAccessibilityNativeError::new(
            WindowsAccessibilityNativeErrorKind::ProviderUnavailable,
        ));
    }
    let event = match kind {
        WindowsAccessibilityNativeEventKind::FocusChanged => EVENT_OBJECT_FOCUS,
        WindowsAccessibilityNativeEventKind::Invoked => EVENT_OBJECT_INVOKED,
        WindowsAccessibilityNativeEventKind::StateChanged => EVENT_OBJECT_STATECHANGE,
        WindowsAccessibilityNativeEventKind::NameChanged => EVENT_OBJECT_NAMECHANGE,
        WindowsAccessibilityNativeEventKind::ValueChanged => EVENT_OBJECT_VALUECHANGE,
        WindowsAccessibilityNativeEventKind::BoundsChanged => EVENT_OBJECT_LOCATIONCHANGE,
        WindowsAccessibilityNativeEventKind::TreeChanged => EVENT_OBJECT_REORDER,
    };
    unsafe {
        NotifyWinEvent(event, null_mut(), OBJID_CLIENT, provider_serial as i32);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn publish_win_event(
    _provider_serial: u64,
    _kind: WindowsAccessibilityNativeEventKind,
) -> Result<(), WindowsAccessibilityNativeError> {
    Err(WindowsAccessibilityNativeError::new(
        WindowsAccessibilityNativeErrorKind::UnsupportedTarget,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_matches_target() {
        let result = WindowsAccessibilityNativeBridge::try_new();
        if cfg!(target_os = "windows") {
            assert!(result.is_ok());
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                WindowsAccessibilityNativeErrorKind::UnsupportedTarget
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn win_event_publication_rejects_invalid_provider_serial() {
        let mut bridge = WindowsAccessibilityNativeBridge::try_new().unwrap();
        assert_eq!(
            bridge
                .publish_event(0, WindowsAccessibilityNativeEventKind::TreeChanged)
                .unwrap_err()
                .kind(),
            WindowsAccessibilityNativeErrorKind::ProviderUnavailable
        );
    }
}
