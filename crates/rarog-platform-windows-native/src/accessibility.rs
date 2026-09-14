use std::fmt;
use std::num::NonZeroIsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeErrorKind {
    UnsupportedTarget,
    InvalidWindow,
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

#[derive(Debug)]
pub struct WindowsAccessibilityNativeBridge {
    hwnd: NonZeroIsize,
}

impl WindowsAccessibilityNativeBridge {
    pub fn try_for_window(hwnd: NonZeroIsize) -> Result<Self, WindowsAccessibilityNativeError> {
        validate_window(hwnd)?;
        Ok(Self { hwnd })
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub fn publish_event(
        &mut self,
        provider_serial: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        publish_win_event(self.hwnd, provider_serial, kind)
    }
}

#[cfg(target_os = "windows")]
fn validate_window(hwnd: NonZeroIsize) -> Result<(), WindowsAccessibilityNativeError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;

    let hwnd = hwnd.get() as HWND;
    if unsafe { IsWindow(hwnd) } == 0 {
        return Err(WindowsAccessibilityNativeError::new(
            WindowsAccessibilityNativeErrorKind::InvalidWindow,
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn validate_window(_hwnd: NonZeroIsize) -> Result<(), WindowsAccessibilityNativeError> {
    Err(WindowsAccessibilityNativeError::new(
        WindowsAccessibilityNativeErrorKind::UnsupportedTarget,
    ))
}

#[cfg(target_os = "windows")]
fn publish_win_event(
    hwnd: NonZeroIsize,
    provider_serial: u64,
    kind: WindowsAccessibilityNativeEventKind,
) -> Result<(), WindowsAccessibilityNativeError> {
    use windows_sys::Win32::Foundation::HWND;
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
    let hwnd = hwnd.get() as HWND;
    unsafe {
        NotifyWinEvent(event, hwnd, OBJID_CLIENT, provider_serial as i32);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn publish_win_event(
    _hwnd: NonZeroIsize,
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
    fn construction_fails_closed_without_a_supported_live_window() {
        let fake = NonZeroIsize::new(1).expect("non-zero test handle");
        let result = WindowsAccessibilityNativeBridge::try_for_window(fake);
        if cfg!(target_os = "windows") {
            assert_eq!(
                result.unwrap_err().kind(),
                WindowsAccessibilityNativeErrorKind::InvalidWindow
            );
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                WindowsAccessibilityNativeErrorKind::UnsupportedTarget
            );
        }
    }
}
