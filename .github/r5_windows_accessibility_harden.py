from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old!r}")
    p.write_text(text.replace(old, new, 1))


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


# Native bridge: a real, validated HWND is mandatory. No ambient COM probe and no NULL HWND events.
write(
    "crates/rarog-platform-windows-native/src/accessibility.rs",
    r'''use std::fmt;
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
        write!(formatter, "Windows accessibility native error: {:?}", self.kind)
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
''',
)

# Native features: no COM is used; real HWND validation requires WindowsAndMessaging.
replace_once(
    "crates/rarog-platform-windows-native/Cargo.toml",
    '  "Win32_System_Com",\n',
    '',
)
replace_once(
    "crates/rarog-platform-windows-native/Cargo.toml",
    '  "Win32_UI_Accessibility",\n]',
    '  "Win32_UI_Accessibility",\n  "Win32_UI_WindowsAndMessaging",\n]',
)

# Windows service construction now requires the attached live HWND.
replace_once(
    "crates/rarog-platform-windows/src/accessibility.rs",
    "use std::num::{NonZeroU64, NonZeroUsize};",
    "use std::num::{NonZeroIsize, NonZeroU64, NonZeroUsize};",
)
replace_once(
    "crates/rarog-platform-windows/src/accessibility.rs",
    "    UnsupportedTarget,\n    InvalidLimits,\n    Native(WindowsAccessibilityNativeErrorKind),",
    "    UnsupportedTarget,\n    InvalidLimits,\n    AlreadyAttached,\n    Native(WindowsAccessibilityNativeErrorKind),",
)
replace_once(
    "crates/rarog-platform-windows/src/accessibility.rs",
    "    pub fn try_new(\n        limits: WindowsAccessibilityLimits,\n    ) -> Result<Self, WindowsAccessibilityBridgeError> {\n        if !Self::target_available() {\n            return Err(WindowsAccessibilityBridgeError::UnsupportedTarget);\n        }\n        let native = WindowsAccessibilityNativeBridge::try_new()\n            .map_err(|error| WindowsAccessibilityBridgeError::Native(error.kind()))?;\n        Ok(Self::with_backend(limits, Box::new(native)))\n    }\n\n    pub fn with_default_limits() -> Result<Self, WindowsAccessibilityBridgeError> {\n        Self::try_new(WindowsAccessibilityLimits::default())\n    }",
    "    pub fn try_for_window(\n        limits: WindowsAccessibilityLimits,\n        hwnd: NonZeroIsize,\n    ) -> Result<Self, WindowsAccessibilityBridgeError> {\n        if !Self::target_available() {\n            return Err(WindowsAccessibilityBridgeError::UnsupportedTarget);\n        }\n        let native = WindowsAccessibilityNativeBridge::try_for_window(hwnd)\n            .map_err(|error| WindowsAccessibilityBridgeError::Native(error.kind()))?;\n        Ok(Self::with_backend(limits, Box::new(native)))\n    }\n\n    pub fn with_default_limits_for_window(\n        hwnd: NonZeroIsize,\n    ) -> Result<Self, WindowsAccessibilityBridgeError> {\n        Self::try_for_window(WindowsAccessibilityLimits::default(), hwnd)\n    }",
)

# Windows host is accessibility-false until a real live window is attached.
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "use std::fmt;",
    "use std::fmt;\nuse std::num::NonZeroIsize;\nuse std::sync::OnceLock;",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "    accessibility: WindowsAccessibilityService,",
    "    accessibility: OnceLock<WindowsAccessibilityService>,",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "                clipboard: WindowsClipboardService::with_default_limits()?,\n                accessibility: WindowsAccessibilityService::with_default_limits()?,",
    "                clipboard: WindowsClipboardService::with_default_limits()?,\n                accessibility: OnceLock::new(),",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "    pub const fn accessibility(&self) -> &WindowsAccessibilityService {\n        &self.accessibility\n    }",
    "    pub fn accessibility(&self) -> Option<&WindowsAccessibilityService> {\n        self.accessibility.get()\n    }\n\n    pub fn attach_accessibility_window(\n        &self,\n        hwnd: NonZeroIsize,\n    ) -> Result<&WindowsAccessibilityService, WindowsAccessibilityBridgeError> {\n        if self.accessibility.get().is_some() {\n            return Err(WindowsAccessibilityBridgeError::AlreadyAttached);\n        }\n        let service = WindowsAccessibilityService::with_default_limits_for_window(hwnd)?;\n        self.accessibility\n            .set(service)\n            .map_err(|_| WindowsAccessibilityBridgeError::AlreadyAttached)?;\n        self.accessibility\n            .get()\n            .ok_or(WindowsAccessibilityBridgeError::AlreadyAttached)\n    }",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            accessibility: true,",
    "            accessibility: self.accessibility.get().is_some(),",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "    fn accessibility_service(&self) -> Option<&dyn PlatformAccessibilityService> {\n        Some(&self.accessibility)\n    }",
    "    fn accessibility_service(&self) -> Option<&dyn PlatformAccessibilityService> {\n        self.accessibility\n            .get()\n            .map(|service| service as &dyn PlatformAccessibilityService)\n    }",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            assert!(host.capabilities().supports(PlatformService::Accessibility));\n            assert!(\n                host.capabilities()",
    "            assert!(!host.capabilities().supports(PlatformService::Accessibility));\n            assert!(\n                host.capabilities()",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            assert!(host.accessibility_service().is_some());",
    "            assert!(host.accessibility_service().is_none());",
)

# Engine builder can share a late-bound platform host with the shell.
replace_once(
    "crates/rarog-engine/src/embedder.rs",
    "    pub fn platform_host<P>(mut self, host: P) -> Self\n    where\n        P: PlatformHost + 'static,\n    {\n        self.platform_host = Arc::new(host);\n        self\n    }",
    "    pub fn platform_host<P>(mut self, host: P) -> Self\n    where\n        P: PlatformHost + 'static,\n    {\n        self.platform_host = Arc::new(host);\n        self\n    }\n\n    pub fn platform_host_arc(mut self, host: Arc<dyn PlatformHost>) -> Self {\n        self.platform_host = host;\n        self\n    }",
)
replace_once(
    "crates/rarog-engine/src/embedder.rs",
    "            frame_scheduler: FrameScheduler::new(),\n        })",
    "            frame_scheduler: FrameScheduler::new(),\n            platform_accessibility_error: None,\n        })",
)
replace_once(
    "crates/rarog-engine/src/embedder.rs",
    "    frame_scheduler: FrameScheduler,\n}",
    "    frame_scheduler: FrameScheduler,\n    platform_accessibility_error: Option<super::PlatformAccessibilityBridgeError>,\n}",
)
replace_once(
    "crates/rarog-engine/src/embedder.rs",
    "    pub fn pending_navigation(&self) -> Option<NavigationId> {\n        self.pending_navigation.as_ref().map(|pending| pending.id)\n    }",
    "    pub fn pending_navigation(&self) -> Option<NavigationId> {\n        self.pending_navigation.as_ref().map(|pending| pending.id)\n    }\n\n    pub const fn platform_accessibility_error(\n        &self,\n    ) -> Option<super::PlatformAccessibilityBridgeError> {\n        self.platform_accessibility_error\n    }",
)
replace_once(
    "crates/rarog-engine/src/embedder.rs",
    "        self.frame_scheduler = FrameScheduler::new();\n        self.frame_scheduler.request(FrameCause::Initial);",
    "        self.frame_scheduler = FrameScheduler::new();\n        self.frame_scheduler.request(FrameCause::Initial);\n        self.platform_accessibility_error = self\n            .shared\n            .platform_host\n            .accessibility_service()\n            .and_then(|service| service.clear_snapshot().err())\n            .map(super::PlatformAccessibilityBridgeError::Platform);",
)
replace_once(
    "crates/rarog-engine/src/embedder.rs",
    "        self.session\n            .as_mut()\n            .expect(\"successful render establishes an active session\")\n            .set_viewport_translation(viewport_translation)?;\n\n        self.shared.event_sink.on_event(&ViewEvent::FrameRendered {",
    "        self.session\n            .as_mut()\n            .expect(\"successful render establishes an active session\")\n            .set_viewport_translation(viewport_translation)?;\n\n        self.platform_accessibility_error = match self.shared.platform_host.accessibility_service() {\n            Some(service) => self\n                .session\n                .as_mut()\n                .expect(\"successful render establishes an active session\")\n                .sync_platform_accessibility(service)\n                .err(),\n            None => None,\n        };\n\n        self.shared.event_sink.on_event(&ViewEvent::FrameRendered {",
)

# If snapshot replacement fails, retire the old platform snapshot rather than exposing it as current.
replace_once(
    "crates/rarog-engine/src/accessibility_platform.rs",
    "        let projection = project_snapshot(snapshot)?;\n        service.replace_snapshot(&projection)?;",
    "        let projection = project_snapshot(snapshot)?;\n        if let Err(error) = service.replace_snapshot(&projection) {\n            let _ = service.clear_snapshot();\n            return Err(error.into());\n        }",
)

# The Windows shell owns the live winit Window, extracts the Win32 handle there, and attaches it.
replace_once(
    "crates/rarog-shell/src/bin/rarog-window.rs",
    "    use rarog_platform_windows::{WindowsGpuError, WindowsPresentingCompositor};",
    "    use rarog_platform_windows::{\n        WindowsGpuError, WindowsPlatformHost, WindowsPresentingCompositor,\n    };",
)
replace_once(
    "crates/rarog-shell/src/bin/rarog-window.rs",
    "    use winit::application::ApplicationHandler;",
    "    use winit::application::ApplicationHandler;\n    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};",
)
replace_once(
    "crates/rarog-shell/src/bin/rarog-window.rs",
    "        let source = fs::read_to_string(&input)?;\n        let engine = Engine::builder().build()?;",
    "        let source = fs::read_to_string(&input)?;\n        let platform = Arc::new(WindowsPlatformHost::try_new()?);\n        let engine = Engine::builder()\n            .platform_host_arc(platform.clone())\n            .build()?;",
)
replace_once(
    "crates/rarog-shell/src/bin/rarog-window.rs",
    "            view,\n            window: None,",
    "            view,\n            platform,\n            window: None,",
)
replace_once(
    "crates/rarog-shell/src/bin/rarog-window.rs",
    "        view: View,\n        window: Option<Arc<Window>>,
",
    "        view: View,\n        platform: Arc<WindowsPlatformHost>,\n        window: Option<Arc<Window>>,
",
)
replace_once(
    "crates/rarog-shell/src/bin/rarog-window.rs",
    "            let window = Arc::new(event_loop.create_window(attributes)?);\n            let size = window.inner_size();",
    "            let window = Arc::new(event_loop.create_window(attributes)?);\n            let window_handle = window.window_handle()?;\n            let hwnd = match window_handle.as_raw() {\n                RawWindowHandle::Win32(handle) => handle.hwnd,\n                _ => return Err(io::Error::other(\"winit did not expose a Win32 HWND\").into()),\n            };\n            self.platform.attach_accessibility_window(hwnd)?;\n            let size = window.inner_size();",
)

print("HWND-bound accessibility hardening applied")
