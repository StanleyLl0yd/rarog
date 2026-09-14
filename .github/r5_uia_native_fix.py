from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
BASE = "72ad8a42790c2a58a6f2d059faccefab33b4d01a"
LIB = ROOT / "crates/rarog-platform-windows-native/src/lib.rs"
UIA = ROOT / "crates/rarog-platform-windows-native/src/accessibility_uia.rs"
MANIFEST = ROOT / "crates/rarog-platform-windows-native/Cargo.toml"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def restore_r4_lib() -> None:
    raw = subprocess.check_output(
        ["git", "show", f"{BASE}:crates/rarog-platform-windows-native/src/lib.rs"],
        cwd=ROOT,
    ).decode("utf-8")
    marker = "#![allow(unsafe_code)]\n"
    if raw.count(marker) != 1:
        raise RuntimeError("native lib base marker changed")
    raw = raw.replace(
        marker,
        marker
        + "\nmod accessibility;\n"
        + "#[cfg(target_os = \"windows\")]\n"
        + "mod accessibility_uia;\n"
        + "pub use accessibility::*;\n",
        1,
    )
    LIB.write_text(raw, encoding="utf-8")


def patch_manifest() -> None:
    text = MANIFEST.read_text(encoding="utf-8")
    if '"Win32_Graphics_Gdi",' not in text:
        text = replace_once(
            text,
            '  "Win32_Foundation",\n  "Win32_Security",',
            '  "Win32_Foundation",\n  "Win32_Graphics_Gdi",\n  "Win32_Security",',
            "windows-sys GDI feature",
        )
    MANIFEST.write_text(text, encoding="utf-8")


def patch_uia() -> None:
    text = UIA.read_text(encoding="utf-8")

    text = replace_once(
        text,
        "    native_error, valid_provider_serial, WindowsAccessibilityNativeAction,\n",
        "    native_error, WindowsAccessibilityNativeAction,\n",
        "test-only provider serial import",
    )
    text = replace_once(
        text,
        "            UIA_BoundingRectanglePropertyId, UIA_ControlTypePropertyId,\n",
        textwrap := "            UIA_ControlTypePropertyId,\n",
        "unused bounding rectangle property import",
    )
    text = replace_once(
        text,
        "    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},\n",
        "    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},\n    Graphics::Gdi::ClientToScreen,\n",
        "GDI ClientToScreen import",
    )
    text = replace_once(
        text,
        "        Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, SAFEARRAY as RawSafeArray},\n",
        "        Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},\n",
        "unused raw SAFEARRAY import",
    )
    text = replace_once(
        text,
        "        WindowsAndMessaging::{ClientToScreen, IsWindow, ScreenToClient, WM_GETOBJECT},\n",
        "        WindowsAndMessaging::{IsWindow, WM_GETOBJECT},\n",
        "window messaging imports",
    )

    dead_methods = '''\n    fn root_serial_win(&self) -> WinResult<u64> {
        self.lock_win()?
            .current
            .as_ref()
            .map(|current| current.root_provider_serial)
            .ok_or_else(element_unavailable_error)
    }

    fn provider_fragment(&self, serial: u64) -> WinResult<IRawElementProviderFragment> {
        let current = self.lock_win()?.current.clone().ok_or_else(element_unavailable_error)?;
        if !current.nodes.contains_key(&serial) {
            return Err(element_unavailable_error());
        }
        if serial == current.root_provider_serial {
            Ok(RootProvider::new(Arc::new(self.clone_for_provider()), serial).into())
        } else {
            Ok(FragmentProvider::new(Arc::new(self.clone_for_provider()), serial).into())
        }
    }

    fn clone_for_provider(&self) -> Self {
        // Providers must share the exact state, never a copied authority. This function is only
        // called through an Arc-owned shared object and is replaced by `provider_*_from_arc` below.
        unreachable!("provider construction must preserve the shared Arc")
    }
'''
    text = replace_once(text, dead_methods, "\n", "dead provider helpers")

    old_enqueue = '''    fn enqueue_action(&self, serial: u64, action: WindowsAccessibilityNativeAction) -> WinResult<()> {
        let mut state = self.lock_win()?;
        let Some(current) = state.current.as_ref() else {
            return Err(element_unavailable_error());
        };
        if !current.nodes.contains_key(&serial) {
            return Err(element_unavailable_error());
        }
        if state.pending_actions.len() >= self.max_pending_actions.get() {
            state.callback_error = Some(WindowsAccessibilityNativeErrorKind::CapacityExceeded);
            return Err(Error::from(E_OUTOFMEMORY));
        }
        state.pending_actions.push_back(WindowsAccessibilityNativeActionRequest::new(
            serial,
            current.document_generation,
            current.geometry_revision,
            action,
        ));
        Ok(())
    }
'''
    new_enqueue = '''    fn enqueue_action(&self, serial: u64, action: WindowsAccessibilityNativeAction) -> WinResult<()> {
        let mut state = self.lock_win()?;
        let (document_generation, geometry_revision) = {
            let Some(current) = state.current.as_ref() else {
                return Err(element_unavailable_error());
            };
            if !current.nodes.contains_key(&serial) {
                return Err(element_unavailable_error());
            }
            (current.document_generation, current.geometry_revision)
        };
        if state.pending_actions.len() >= self.max_pending_actions.get() {
            state.callback_error = Some(WindowsAccessibilityNativeErrorKind::CapacityExceeded);
            return Err(Error::from(E_OUTOFMEMORY));
        }
        state.pending_actions.push_back(WindowsAccessibilityNativeActionRequest::new(
            serial,
            document_generation,
            geometry_revision,
            action,
        ));
        Ok(())
    }
'''
    text = replace_once(text, old_enqueue, new_enqueue, "enqueue borrow split")

    old_replace = '''        let old = state.current.replace(candidate);
        state.previous = old;
        if state
            .focused_provider
            .is_some_and(|serial| state.current.as_ref().is_none_or(|current| !current.nodes.contains_key(&serial)))
        {
            state.focused_provider = None;
        }
        state.pending_actions.retain(|request| {
            state.current.as_ref().is_some_and(|current| {
                request.document_generation() == current.document_generation
                    && request.geometry_revision() == current.geometry_revision
                    && current.nodes.contains_key(&request.provider_serial())
            })
        });
'''
    new_replace = '''        let old = state.current.replace(candidate);
        state.previous = old;
        let (document_generation, geometry_revision, provider_serials) = {
            let Some(current) = state.current.as_ref() else {
                return Err(native_error(WindowsAccessibilityNativeErrorKind::ProviderUnavailable));
            };
            (
                current.document_generation,
                current.geometry_revision,
                current.nodes.keys().copied().collect::<std::collections::BTreeSet<_>>(),
            )
        };
        if state
            .focused_provider
            .is_some_and(|serial| !provider_serials.contains(&serial))
        {
            state.focused_provider = None;
        }
        state.pending_actions.retain(|request| {
            request.document_generation() == document_generation
                && request.geometry_revision() == geometry_revision
                && provider_serials.contains(&request.provider_serial())
        });
'''
    text = replace_once(text, old_replace, new_replace, "snapshot retain borrow split")

    old_publish = '''            if kind == WindowsAccessibilityNativeEventKind::FocusChanged {
                state.focused_provider = Some(provider_serial);
            }
            let current_node = current.nodes.get(&provider_serial).cloned();
            let previous_node = state
                .previous
                .as_ref()
                .and_then(|previous| previous.nodes.get(&provider_serial))
                .cloned();
            (current_node, previous_node)
'''
    new_publish = '''            let current_node = current.nodes.get(&provider_serial).cloned();
            let previous_node = state
                .previous
                .as_ref()
                .and_then(|previous| previous.nodes.get(&provider_serial))
                .cloned();
            if kind == WindowsAccessibilityNativeEventKind::FocusChanged {
                state.focused_provider = Some(provider_serial);
            }
            (current_node, previous_node)
'''
    text = replace_once(text, old_publish, new_publish, "event focus borrow split")

    null_replacements = {
        "        Ok(IRawElementProviderSimple::default())\n": "        Err(Error::empty())\n",
        "            None => Ok(IRawElementProviderFragment::default()),\n": "            None => Err(Error::empty()),\n",
        "    Ok(IUnknown::default())\n": "    Err(Error::empty())\n",
        "            let Some(parent) = node.parent() else { return Ok(IRawElementProviderFragment::default()); };\n": "            let Some(parent) = node.parent() else { return Err(Error::empty()); };\n",
        "        None => Ok(IRawElementProviderFragment::default()),\n": "        None => Err(Error::empty()),\n",
    }
    expected_counts = {
        "        Ok(IRawElementProviderSimple::default())\n": 1,
        "            None => Ok(IRawElementProviderFragment::default()),\n": 2,
        "    Ok(IUnknown::default())\n": 1,
        "            let Some(parent) = node.parent() else { return Ok(IRawElementProviderFragment::default()); };\n": 1,
        "        None => Ok(IRawElementProviderFragment::default()),\n": 1,
    }
    for old, new in null_replacements.items():
        count = text.count(old)
        expected = expected_counts[old]
        if count != expected:
            raise RuntimeError(f"nullable COM return {old!r}: expected {expected}, found {count}")
        text = text.replace(old, new)

    dead_screen = '''\n#[allow(dead_code)]
fn screen_to_client(hwnd: NonZeroIsize, x: i32, y: i32) -> WinResult<POINT> {
    let mut point = POINT { x, y };
    if unsafe { ScreenToClient(raw_hwnd(hwnd), &mut point) } == 0 {
        return Err(element_unavailable_error());
    }
    Ok(point)
}
'''
    text = replace_once(text, dead_screen, "\n", "dead ScreenToClient helper")
    text = replace_once(
        text,
        "    TypedHwnd(hwnd as isize)\n",
        "    TypedHwnd(hwnd)\n",
        "typed HWND pointer representation",
    )
    text = text.replace(
        "valid_provider_serial(",
        "crate::accessibility::valid_provider_serial(",
    )

    UIA.write_text(text, encoding="utf-8")


restore_r4_lib()
patch_manifest()
patch_uia()

# Guard the R4 sandbox invariant explicitly before any Windows compiler claim.
lib = LIB.read_text(encoding="utf-8")
if "JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE" not in lib:
    raise RuntimeError("R4 sandbox invariant lost: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE missing")
if "JOB_OBJECT_LIMIT_KILL_ON_CLOSE" in lib:
    raise RuntimeError("invalid shortened job-object constant remains")
