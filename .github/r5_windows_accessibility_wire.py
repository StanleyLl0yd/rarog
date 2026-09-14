from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one match, found {count}: {old!r}")
    p.write_text(text.replace(old, new, 1))


# rarog-platform crate root
replace_once(
    "crates/rarog-platform/src/lib.rs",
    "pub mod clipboard;\npub mod input;\n\npub use clipboard::{",
    "pub mod accessibility;\npub mod clipboard;\npub mod input;\n\npub use accessibility::*;\npub use clipboard::{",
)
replace_once(
    "crates/rarog-platform/src/lib.rs",
    "    fn clipboard_service(&self) -> Option<&dyn PlatformClipboardService> {\n        None\n    }\n}",
    "    fn clipboard_service(&self) -> Option<&dyn PlatformClipboardService> {\n        None\n    }\n\n    fn accessibility_service(&self) -> Option<&dyn PlatformAccessibilityService> {\n        None\n    }\n}",
)
replace_once(
    "crates/rarog-platform/src/lib.rs",
    "        assert!(host.clipboard_service().is_none());\n        for service in [",
    "        assert!(host.clipboard_service().is_none());\n        assert!(host.accessibility_service().is_none());\n        for service in [",
)

# Windows native crate feature/module wiring.
replace_once(
    "crates/rarog-platform-windows-native/Cargo.toml",
    '  "Win32_System_JobObjects",\n  "Win32_System_SystemServices",',
    '  "Win32_System_Com",\n  "Win32_System_JobObjects",\n  "Win32_System_SystemServices",',
)
replace_once(
    "crates/rarog-platform-windows-native/Cargo.toml",
    '  "Win32_System_Threading",\n]',
    '  "Win32_System_Threading",\n  "Win32_UI_Accessibility",\n]',
)
replace_once(
    "crates/rarog-platform-windows-native/src/lib.rs",
    "#![allow(unsafe_code)]\n#![deny(unsafe_op_in_unsafe_fn)]\n\nuse std::ffi",
    "#![allow(unsafe_code)]\n#![deny(unsafe_op_in_unsafe_fn)]\n\nmod accessibility;\npub use accessibility::*;\n\nuse std::ffi",
)

# Windows wrapper crate root/host wiring.
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "mod clipboard;\nmod gpu;",
    "mod accessibility;\nmod clipboard;\nmod gpu;",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "pub use clipboard::WindowsClipboardService;",
    "pub use accessibility::{\n    DEFAULT_MAX_WINDOWS_ACCESSIBILITY_ACTION_REQUESTS,\n    DEFAULT_MAX_WINDOWS_ACCESSIBILITY_PENDING_EVENTS,\n    DEFAULT_MAX_WINDOWS_ACCESSIBILITY_PROVIDERS, WindowsAccessibilityBridgeError,\n    WindowsAccessibilityLimits, WindowsAccessibilityService,\n};\npub use clipboard::WindowsClipboardService;",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "    ClipboardError, PlatformCapabilities, PlatformClipboardService, PlatformFontError,\n    PlatformFontRequest, PlatformFontService, PlatformHost, PlatformInputService,\n    PlatformTextInputService, ResolvedPlatformFont,",
    "    ClipboardError, PlatformAccessibilityService, PlatformCapabilities,\n    PlatformClipboardService, PlatformFontError, PlatformFontRequest, PlatformFontService,\n    PlatformHost, PlatformInputService, PlatformTextInputService, ResolvedPlatformFont,",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "pub enum WindowsPlatformError {\n    UnsupportedTarget,\n    Clipboard(ClipboardError),\n}",
    "pub enum WindowsPlatformError {\n    UnsupportedTarget,\n    Clipboard(ClipboardError),\n    Accessibility(WindowsAccessibilityBridgeError),\n}",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            Self::Clipboard(error) => write!(\n                formatter,\n                \"Windows clipboard initialization failed: {error}\"\n            ),\n        }",
    "            Self::Clipboard(error) => write!(\n                formatter,\n                \"Windows clipboard initialization failed: {error}\"\n            ),\n            Self::Accessibility(error) => write!(\n                formatter,\n                \"Windows accessibility initialization failed: {error}\"\n            ),\n        }",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "impl From<ClipboardError> for WindowsPlatformError {\n    fn from(error: ClipboardError) -> Self {\n        Self::Clipboard(error)\n    }\n}\n",
    "impl From<ClipboardError> for WindowsPlatformError {\n    fn from(error: ClipboardError) -> Self {\n        Self::Clipboard(error)\n    }\n}\n\nimpl From<WindowsAccessibilityBridgeError> for WindowsPlatformError {\n    fn from(error: WindowsAccessibilityBridgeError) -> Self {\n        Self::Accessibility(error)\n    }\n}\n",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "pub struct WindowsPlatformHost {\n    fonts: WindowsFontService,\n    input: WindowsInputService,\n    clipboard: WindowsClipboardService,\n}",
    "pub struct WindowsPlatformHost {\n    fonts: WindowsFontService,\n    input: WindowsInputService,\n    clipboard: WindowsClipboardService,\n    accessibility: WindowsAccessibilityService,\n}",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "                fonts: WindowsFontService::new(),\n                input: WindowsInputService::try_new()?,\n                clipboard: WindowsClipboardService::with_default_limits()?,",
    "                fonts: WindowsFontService::new(),\n                input: WindowsInputService::try_new()?,\n                clipboard: WindowsClipboardService::with_default_limits()?,\n                accessibility: WindowsAccessibilityService::with_default_limits()?,",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "    pub const fn clipboard(&self) -> &WindowsClipboardService {\n        &self.clipboard\n    }\n\n    pub async fn request_gpu",
    "    pub const fn clipboard(&self) -> &WindowsClipboardService {\n        &self.clipboard\n    }\n\n    pub const fn accessibility(&self) -> &WindowsAccessibilityService {\n        &self.accessibility\n    }\n\n    pub async fn request_gpu",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            clipboard: true,\n            sandbox_process: true,",
    "            clipboard: true,\n            accessibility: true,\n            sandbox_process: true,",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "    fn clipboard_service(&self) -> Option<&dyn PlatformClipboardService> {\n        Some(&self.clipboard)\n    }\n}",
    "    fn clipboard_service(&self) -> Option<&dyn PlatformClipboardService> {\n        Some(&self.clipboard)\n    }\n\n    fn accessibility_service(&self) -> Option<&dyn PlatformAccessibilityService> {\n        Some(&self.accessibility)\n    }\n}",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            assert!(host.capabilities().supports(PlatformService::Clipboard));\n            assert!(\n                host.capabilities()",
    "            assert!(host.capabilities().supports(PlatformService::Clipboard));\n            assert!(host.capabilities().supports(PlatformService::Accessibility));\n            assert!(\n                host.capabilities()",
)
replace_once(
    "crates/rarog-platform-windows/src/lib.rs",
    "            assert!(host.clipboard_service().is_some());\n        } else {",
    "            assert!(host.clipboard_service().is_some());\n            assert!(host.accessibility_service().is_some());\n        } else {",
)

# Engine module/export wiring.
replace_once(
    "crates/rarog-engine/src/lib.rs",
    "mod accessibility;\nmod embedder;",
    "mod accessibility;\nmod accessibility_platform;\nmod embedder;",
)
replace_once(
    "crates/rarog-engine/src/lib.rs",
    "pub use embedder::*;\npub use event_loop::*;",
    "pub use accessibility_platform::*;\npub use embedder::*;\npub use event_loop::*;",
)

# Expose the existing #312 engine action authority through RenderSession.
replace_once(
    "crates/rarog-engine/src/accessibility.rs",
    "use rarog_accessibility::{\n    AccessibilityEvent, AccessibilityInvalidation, AccessibilityRefreshError, AccessibilityRuntime,\n    AccessibilitySnapshot,\n};",
    "use rarog_accessibility::{\n    AccessibilityAction, AccessibilityActionError, AccessibilityActionExecutor,\n    AccessibilityActionResult, AccessibilityEvent, AccessibilityInvalidation, AccessibilityNodeId,\n    AccessibilityRefreshError, AccessibilityRuntime, AccessibilitySnapshot,\n};",
)
replace_once(
    "crates/rarog-engine/src/accessibility.rs",
    "    pub fn accessibility_geometry_revision(&self) -> u64 {\n        self.accessibility.geometry_revision\n    }\n\n    pub fn resize",
    "    pub fn accessibility_geometry_revision(&self) -> u64 {\n        self.accessibility.geometry_revision\n    }\n\n    pub fn perform_accessibility_action<E: AccessibilityActionExecutor>(\n        &mut self,\n        executor: &mut E,\n        target: AccessibilityNodeId,\n        action: AccessibilityAction,\n    ) -> Result<AccessibilityActionResult, AccessibilityActionError> {\n        let runtime = self\n            .accessibility\n            .runtime\n            .as_mut()\n            .ok_or(AccessibilityActionError::InconsistentAuthority)?;\n        runtime.perform_action(&mut self.document, executor, target, action)\n    }\n\n    pub fn resize",
)

print("Windows accessibility bridge wiring applied")
