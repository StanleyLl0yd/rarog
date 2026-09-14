use crate::accessibility::{
    WindowsAccessibilityNativeAction, WindowsAccessibilityNativeActionRequest,
    WindowsAccessibilityNativeError, WindowsAccessibilityNativeErrorKind,
    WindowsAccessibilityNativeEventKind, WindowsAccessibilityNativeNode,
    WindowsAccessibilityNativeRole, WindowsAccessibilityNativeSnapshot, native_error,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::num::{NonZeroIsize, NonZeroUsize};
use std::sync::{Arc, Mutex, MutexGuard};
use windows::{
    Win32::{
        Foundation::{HWND as TypedHwnd, LPARAM as TypedLparam, WPARAM as TypedWparam},
        System::Com::SAFEARRAY,
        UI::Accessibility::{
            ExpandCollapseState, ExpandCollapseState_Collapsed, ExpandCollapseState_Expanded,
            IExpandCollapseProvider, IExpandCollapseProvider_Impl, IInvokeProvider,
            IInvokeProvider_Impl, IRawElementProviderFragment, IRawElementProviderFragment_Impl,
            IRawElementProviderFragmentRoot, IRawElementProviderFragmentRoot_Impl,
            IRawElementProviderSimple, IRawElementProviderSimple_Impl, IToggleProvider,
            IToggleProvider_Impl, NavigateDirection, NavigateDirection_FirstChild,
            NavigateDirection_LastChild, NavigateDirection_NextSibling, NavigateDirection_Parent,
            NavigateDirection_PreviousSibling, ProviderOptions, ProviderOptions_ServerSideProvider,
            StructureChangeType_ChildrenInvalidated, ToggleState, ToggleState_Off, ToggleState_On,
            UIA_AutomationFocusChangedEventId, UIA_ControlTypePropertyId,
            UIA_ExpandCollapseExpandCollapseStatePropertyId, UIA_ExpandCollapsePatternId,
            UIA_Invoke_InvokedEventId, UIA_InvokePatternId, UIA_IsEnabledPropertyId,
            UIA_IsKeyboardFocusablePropertyId, UIA_LayoutInvalidatedEventId, UIA_NamePropertyId,
            UIA_PATTERN_ID, UIA_PROPERTY_ID, UIA_TogglePatternId, UIA_ToggleToggleStatePropertyId,
            UiaAppendRuntimeId, UiaHostProviderFromHwnd, UiaRaiseAutomationEvent,
            UiaRaiseAutomationPropertyChangedEvent, UiaRaiseStructureChangedEvent, UiaRect,
            UiaReturnRawElementProvider, UiaRootObjectId,
        },
    },
    core::{BSTR, Error, HRESULT, IUnknown, Interface, Result as WinResult, VARIANT, implement},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::ClientToScreen,
    System::{
        Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        Ole::{SafeArrayCreateVector, SafeArrayDestroy, SafeArrayPutElement},
        Variant::VT_I4,
    },
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{IsWindow, WM_GETOBJECT},
    },
};

const SUBCLASS_ID: usize = 0x5241_524F_4705;
const E_NOTIMPL: HRESULT = HRESULT(0x8000_4001_u32 as i32);
const E_OUTOFMEMORY: HRESULT = HRESULT(0x8007_000E_u32 as i32);
const UIA_E_ELEMENTNOTAVAILABLE_HRESULT: HRESULT = HRESULT(0x8004_0201_u32 as i32);
const UIA_E_NOTSUPPORTED_HRESULT: HRESULT = HRESULT(0x8004_0204_u32 as i32);
const RPC_E_CHANGED_MODE: i32 = 0x8001_0106_u32 as i32;

#[derive(Clone, Debug)]
struct SnapshotState {
    document_generation: u64,
    geometry_revision: u64,
    root_provider_serial: u64,
    nodes: BTreeMap<u64, WindowsAccessibilityNativeNode>,
}

impl SnapshotState {
    fn from_snapshot(snapshot: &WindowsAccessibilityNativeSnapshot) -> Self {
        Self {
            document_generation: snapshot.document_generation(),
            geometry_revision: snapshot.geometry_revision(),
            root_provider_serial: snapshot.root_provider_serial(),
            nodes: snapshot
                .nodes()
                .iter()
                .cloned()
                .map(|node| (node.provider_serial(), node))
                .collect(),
        }
    }
}

#[derive(Debug)]
struct UiaState {
    current: Option<SnapshotState>,
    previous: Option<SnapshotState>,
    focused_provider: Option<u64>,
    pending_actions: VecDeque<WindowsAccessibilityNativeActionRequest>,
    callback_error: Option<WindowsAccessibilityNativeErrorKind>,
}

#[derive(Debug)]
struct UiaShared {
    hwnd: NonZeroIsize,
    max_pending_actions: NonZeroUsize,
    state: Mutex<UiaState>,
}

impl UiaShared {
    fn lock(&self) -> Result<MutexGuard<'_, UiaState>, WindowsAccessibilityNativeError> {
        self.state
            .lock()
            .map_err(|_| native_error(WindowsAccessibilityNativeErrorKind::ComFailure))
    }

    fn lock_win(&self) -> WinResult<MutexGuard<'_, UiaState>> {
        self.state.lock().map_err(|_| Error::from(E_NOTIMPL))
    }

    fn current_node_win(
        &self,
        serial: u64,
    ) -> WinResult<(SnapshotState, WindowsAccessibilityNativeNode)> {
        let state = self.lock_win()?;
        let current = state
            .current
            .as_ref()
            .ok_or_else(element_unavailable_error)?;
        let node = current
            .nodes
            .get(&serial)
            .cloned()
            .ok_or_else(element_unavailable_error)?;
        Ok((current.clone(), node))
    }

    fn enqueue_action(
        &self,
        serial: u64,
        action: WindowsAccessibilityNativeAction,
    ) -> WinResult<()> {
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
        state
            .pending_actions
            .push_back(WindowsAccessibilityNativeActionRequest::new(
                serial,
                document_generation,
                geometry_revision,
                action,
            ));
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct WindowsUiaBridge {
    hwnd: NonZeroIsize,
    shared: Arc<UiaShared>,
    com_uninitialize: bool,
}

impl WindowsUiaBridge {
    pub(crate) fn try_new(
        hwnd: NonZeroIsize,
        max_pending_actions: NonZeroUsize,
    ) -> Result<Self, WindowsAccessibilityNativeError> {
        validate_window(hwnd)?;
        let com_result =
            unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        let com_uninitialize = if com_result >= 0 {
            true
        } else if com_result == RPC_E_CHANGED_MODE {
            false
        } else {
            return Err(native_error(
                WindowsAccessibilityNativeErrorKind::ComFailure,
            ));
        };
        let shared = Arc::new(UiaShared {
            hwnd,
            max_pending_actions,
            state: Mutex::new(UiaState {
                current: None,
                previous: None,
                focused_provider: None,
                pending_actions: VecDeque::new(),
                callback_error: None,
            }),
        });
        let raw_hwnd = raw_hwnd(hwnd);
        let installed = unsafe {
            SetWindowSubclass(
                raw_hwnd,
                Some(uia_subclass_proc),
                SUBCLASS_ID,
                Arc::as_ptr(&shared) as usize,
            )
        };
        if installed == 0 {
            if com_uninitialize {
                unsafe { CoUninitialize() };
            }
            return Err(native_error(
                WindowsAccessibilityNativeErrorKind::ComFailure,
            ));
        }
        Ok(Self {
            hwnd,
            shared,
            com_uninitialize,
        })
    }

    pub(crate) fn clear_snapshot(&mut self) -> Result<(), WindowsAccessibilityNativeError> {
        validate_window(self.hwnd)?;
        let mut state = self.shared.lock()?;
        state.current = None;
        state.previous = None;
        state.focused_provider = None;
        state.pending_actions.clear();
        state.callback_error = None;
        Ok(())
    }

    pub(crate) fn replace_snapshot(
        &mut self,
        snapshot: &WindowsAccessibilityNativeSnapshot,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        validate_window(self.hwnd)?;
        let candidate = SnapshotState::from_snapshot(snapshot);
        let mut state = self.shared.lock()?;
        let old = state.current.replace(candidate);
        state.previous = old;
        let (document_generation, geometry_revision, provider_serials) = {
            let Some(current) = state.current.as_ref() else {
                return Err(native_error(
                    WindowsAccessibilityNativeErrorKind::ProviderUnavailable,
                ));
            };
            (
                current.document_generation,
                current.geometry_revision,
                current
                    .nodes
                    .keys()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
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
        state.callback_error = None;
        Ok(())
    }

    pub(crate) fn next_action_request(
        &mut self,
    ) -> Result<Option<WindowsAccessibilityNativeActionRequest>, WindowsAccessibilityNativeError>
    {
        validate_window(self.hwnd)?;
        let mut state = self.shared.lock()?;
        if let Some(error) = state.callback_error.take() {
            return Err(native_error(error));
        }
        Ok(state.pending_actions.pop_front())
    }

    pub(crate) fn publish_event_current(
        &mut self,
        provider_serial: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        let (document_generation, geometry_revision) = {
            let state = self.shared.lock()?;
            let current = state.current.as_ref().ok_or_else(|| {
                native_error(WindowsAccessibilityNativeErrorKind::ProviderUnavailable)
            })?;
            (current.document_generation, current.geometry_revision)
        };
        self.publish_event(
            provider_serial,
            document_generation,
            geometry_revision,
            kind,
        )
    }

    pub(crate) fn publish_event(
        &mut self,
        provider_serial: u64,
        document_generation: u64,
        geometry_revision: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        validate_window(self.hwnd)?;
        let event_data = {
            let state = self.shared.lock()?;
            let current = state.current.as_ref().ok_or_else(|| {
                native_error(WindowsAccessibilityNativeErrorKind::ProviderUnavailable)
            })?;
            if current.document_generation != document_generation
                || current.geometry_revision != geometry_revision
                || !current.nodes.contains_key(&provider_serial)
            {
                return Err(native_error(
                    WindowsAccessibilityNativeErrorKind::ProviderUnavailable,
                ));
            }
            let current_node = current.nodes.get(&provider_serial).cloned();
            let previous_node = state
                .previous
                .as_ref()
                .and_then(|previous| previous.nodes.get(&provider_serial))
                .cloned();
            (current_node, previous_node)
        };
        let provider = simple_provider_from_arc(&self.shared, provider_serial)?;
        raise_event(provider, kind, event_data.0.as_ref(), event_data.1.as_ref())?;
        if kind == WindowsAccessibilityNativeEventKind::FocusChanged {
            let mut state = self.shared.lock()?;
            let current = state.current.as_ref().ok_or_else(|| {
                native_error(WindowsAccessibilityNativeErrorKind::ProviderUnavailable)
            })?;
            if current.document_generation != document_generation
                || current.geometry_revision != geometry_revision
                || !current.nodes.contains_key(&provider_serial)
            {
                return Err(native_error(
                    WindowsAccessibilityNativeErrorKind::ProviderUnavailable,
                ));
            }
            state.focused_provider = Some(provider_serial);
        }
        Ok(())
    }
}

impl Drop for WindowsUiaBridge {
    fn drop(&mut self) {
        unsafe {
            RemoveWindowSubclass(raw_hwnd(self.hwnd), Some(uia_subclass_proc), SUBCLASS_ID);
            if self.com_uninitialize {
                CoUninitialize();
            }
        }
    }
}

unsafe extern "system" fn uia_subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    if message == WM_GETOBJECT && lparam == UiaRootObjectId as LPARAM && ref_data != 0 {
        let shared_ptr = ref_data as *const UiaShared;
        unsafe { Arc::increment_strong_count(shared_ptr) };
        let shared = unsafe { Arc::from_raw(shared_ptr) };
        let root = {
            let state = match shared.state.lock() {
                Ok(state) => state,
                Err(_) => return unsafe { DefSubclassProc(hwnd, message, wparam, lparam) },
            };
            state
                .current
                .as_ref()
                .map(|current| current.root_provider_serial)
        };
        if let Some(root) = root {
            if let Ok(provider) = simple_provider_from_arc(&shared, root) {
                let result = unsafe {
                    UiaReturnRawElementProvider(
                        typed_hwnd(hwnd),
                        TypedWparam(wparam),
                        TypedLparam(lparam),
                        &provider,
                    )
                };
                return result.0;
            }
        }
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[derive(Clone)]
struct ProviderCore {
    shared: Arc<UiaShared>,
    serial: u64,
}

impl fmt::Debug for ProviderCore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderCore")
            .field("serial", &self.serial)
            .finish()
    }
}

#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IRawElementProviderFragmentRoot
)]
struct RootProvider {
    core: ProviderCore,
}

impl RootProvider {
    fn new(shared: Arc<UiaShared>, serial: u64) -> Self {
        Self {
            core: ProviderCore { shared, serial },
        }
    }
}

#[implement(IRawElementProviderSimple, IRawElementProviderFragment)]
struct FragmentProvider {
    core: ProviderCore,
}

impl FragmentProvider {
    fn new(shared: Arc<UiaShared>, serial: u64) -> Self {
        Self {
            core: ProviderCore { shared, serial },
        }
    }
}

impl IRawElementProviderSimple_Impl for RootProvider_Impl {
    fn ProviderOptions(&self) -> WinResult<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> WinResult<IUnknown> {
        pattern_provider(&self.core.shared, self.core.serial, pattern)
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> WinResult<VARIANT> {
        property_value(&self.core.shared, self.core.serial, property)
    }

    fn HostRawElementProvider(&self) -> WinResult<IRawElementProviderSimple> {
        unsafe { UiaHostProviderFromHwnd(typed_hwnd(raw_hwnd(self.core.shared.hwnd))) }
    }
}

impl IRawElementProviderSimple_Impl for FragmentProvider_Impl {
    fn ProviderOptions(&self) -> WinResult<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> WinResult<IUnknown> {
        pattern_provider(&self.core.shared, self.core.serial, pattern)
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> WinResult<VARIANT> {
        property_value(&self.core.shared, self.core.serial, property)
    }

    fn HostRawElementProvider(&self) -> WinResult<IRawElementProviderSimple> {
        Err(Error::empty())
    }
}

impl IRawElementProviderFragment_Impl for RootProvider_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> WinResult<IRawElementProviderFragment> {
        navigate(&self.core.shared, self.core.serial, direction)
    }

    fn GetRuntimeId(&self) -> WinResult<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn BoundingRectangle(&self) -> WinResult<UiaRect> {
        bounding_rectangle(&self.core.shared, self.core.serial)
    }

    fn GetEmbeddedFragmentRoots(&self) -> WinResult<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn SetFocus(&self) -> WinResult<()> {
        set_focus(&self.core.shared, self.core.serial)
    }

    fn FragmentRoot(&self) -> WinResult<IRawElementProviderFragmentRoot> {
        root_provider_from_arc(&self.core.shared)
    }
}

impl IRawElementProviderFragment_Impl for FragmentProvider_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> WinResult<IRawElementProviderFragment> {
        navigate(&self.core.shared, self.core.serial, direction)
    }

    fn GetRuntimeId(&self) -> WinResult<*mut SAFEARRAY> {
        runtime_id(self.core.serial)
    }

    fn BoundingRectangle(&self) -> WinResult<UiaRect> {
        bounding_rectangle(&self.core.shared, self.core.serial)
    }

    fn GetEmbeddedFragmentRoots(&self) -> WinResult<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn SetFocus(&self) -> WinResult<()> {
        set_focus(&self.core.shared, self.core.serial)
    }

    fn FragmentRoot(&self) -> WinResult<IRawElementProviderFragmentRoot> {
        root_provider_from_arc(&self.core.shared)
    }
}

impl IRawElementProviderFragmentRoot_Impl for RootProvider_Impl {
    fn ElementProviderFromPoint(&self, x: f64, y: f64) -> WinResult<IRawElementProviderFragment> {
        let serial = hit_test(&self.core.shared, x, y)?;
        match serial {
            Some(serial) => fragment_provider_from_arc(&self.core.shared, serial),
            None => Err(Error::empty()),
        }
    }

    fn GetFocus(&self) -> WinResult<IRawElementProviderFragment> {
        let serial = self.core.shared.lock_win()?.focused_provider;
        match serial {
            Some(serial) => fragment_provider_from_arc(&self.core.shared, serial),
            None => Err(Error::empty()),
        }
    }
}

#[implement(IInvokeProvider)]
struct InvokePattern {
    core: ProviderCore,
}

impl IInvokeProvider_Impl for InvokePattern_Impl {
    fn Invoke(&self) -> WinResult<()> {
        self.core
            .shared
            .enqueue_action(self.core.serial, WindowsAccessibilityNativeAction::Invoke)
    }
}

#[implement(IToggleProvider)]
struct TogglePattern {
    core: ProviderCore,
}

impl IToggleProvider_Impl for TogglePattern_Impl {
    fn Toggle(&self) -> WinResult<()> {
        self.core
            .shared
            .enqueue_action(self.core.serial, WindowsAccessibilityNativeAction::Toggle)
    }

    fn ToggleState(&self) -> WinResult<ToggleState> {
        let (_, node) = self.core.shared.current_node_win(self.core.serial)?;
        match node.checked() {
            Some(true) => Ok(ToggleState_On),
            Some(false) => Ok(ToggleState_Off),
            None => Err(Error::from(UIA_E_NOTSUPPORTED_HRESULT)),
        }
    }
}

#[implement(IExpandCollapseProvider)]
struct ExpandCollapsePattern {
    core: ProviderCore,
}

impl IExpandCollapseProvider_Impl for ExpandCollapsePattern_Impl {
    fn Expand(&self) -> WinResult<()> {
        self.core.shared.enqueue_action(
            self.core.serial,
            WindowsAccessibilityNativeAction::SetExpanded(true),
        )
    }

    fn Collapse(&self) -> WinResult<()> {
        self.core.shared.enqueue_action(
            self.core.serial,
            WindowsAccessibilityNativeAction::SetExpanded(false),
        )
    }

    fn ExpandCollapseState(&self) -> WinResult<ExpandCollapseState> {
        let (_, node) = self.core.shared.current_node_win(self.core.serial)?;
        match node.expanded() {
            Some(true) => Ok(ExpandCollapseState_Expanded),
            Some(false) => Ok(ExpandCollapseState_Collapsed),
            None => Err(Error::from(UIA_E_NOTSUPPORTED_HRESULT)),
        }
    }
}

fn simple_provider_from_arc(
    shared: &Arc<UiaShared>,
    serial: u64,
) -> Result<IRawElementProviderSimple, WindowsAccessibilityNativeError> {
    let state = shared.lock()?;
    let current = state
        .current
        .as_ref()
        .ok_or_else(|| native_error(WindowsAccessibilityNativeErrorKind::ProviderUnavailable))?;
    if !current.nodes.contains_key(&serial) {
        return Err(native_error(
            WindowsAccessibilityNativeErrorKind::ProviderUnavailable,
        ));
    }
    let provider = if serial == current.root_provider_serial {
        let provider: IRawElementProviderSimple =
            RootProvider::new(Arc::clone(shared), serial).into();
        provider
    } else {
        let provider: IRawElementProviderSimple =
            FragmentProvider::new(Arc::clone(shared), serial).into();
        provider
    };
    Ok(provider)
}

fn fragment_provider_from_arc(
    shared: &Arc<UiaShared>,
    serial: u64,
) -> WinResult<IRawElementProviderFragment> {
    let state = shared.lock_win()?;
    let current = state
        .current
        .as_ref()
        .ok_or_else(element_unavailable_error)?;
    if !current.nodes.contains_key(&serial) {
        return Err(element_unavailable_error());
    }
    if serial == current.root_provider_serial {
        let provider: IRawElementProviderFragment =
            RootProvider::new(Arc::clone(shared), serial).into();
        Ok(provider)
    } else {
        let provider: IRawElementProviderFragment =
            FragmentProvider::new(Arc::clone(shared), serial).into();
        Ok(provider)
    }
}

fn root_provider_from_arc(shared: &Arc<UiaShared>) -> WinResult<IRawElementProviderFragmentRoot> {
    let serial = {
        let state = shared.lock_win()?;
        state
            .current
            .as_ref()
            .map(|current| current.root_provider_serial)
            .ok_or_else(element_unavailable_error)?
    };
    let provider: IRawElementProviderFragmentRoot =
        RootProvider::new(Arc::clone(shared), serial).into();
    Ok(provider)
}

fn property_value(
    shared: &Arc<UiaShared>,
    serial: u64,
    property: UIA_PROPERTY_ID,
) -> WinResult<VARIANT> {
    let (_, node) = shared.current_node_win(serial)?;
    if property == UIA_NamePropertyId {
        return Ok(VARIANT::from(BSTR::from(node.name())));
    }
    if property == UIA_ControlTypePropertyId {
        return Ok(VARIANT::from(control_type(node.role())));
    }
    if property == UIA_IsEnabledPropertyId {
        return Ok(VARIANT::from(!node.disabled()));
    }
    if property == UIA_IsKeyboardFocusablePropertyId {
        return Ok(VARIANT::from(node.focusable()));
    }
    if property == UIA_ToggleToggleStatePropertyId {
        return node
            .checked()
            .map(|checked| VARIANT::from(toggle_state_value(checked)))
            .ok_or_else(|| Error::from(UIA_E_NOTSUPPORTED_HRESULT));
    }
    if property == UIA_ExpandCollapseExpandCollapseStatePropertyId {
        return node
            .expanded()
            .map(|expanded| VARIANT::from(expand_state_value(expanded)))
            .ok_or_else(|| Error::from(UIA_E_NOTSUPPORTED_HRESULT));
    }
    Ok(VARIANT::default())
}

fn pattern_provider(
    shared: &Arc<UiaShared>,
    serial: u64,
    pattern: UIA_PATTERN_ID,
) -> WinResult<IUnknown> {
    let (_, node) = shared.current_node_win(serial)?;
    let core = ProviderCore {
        shared: Arc::clone(shared),
        serial,
    };
    if pattern == UIA_InvokePatternId
        && matches!(
            node.role(),
            WindowsAccessibilityNativeRole::Button | WindowsAccessibilityNativeRole::Link
        )
    {
        let provider: IInvokeProvider = InvokePattern { core }.into();
        return provider.cast();
    }
    if pattern == UIA_TogglePatternId && node.checked().is_some() {
        let provider: IToggleProvider = TogglePattern { core }.into();
        return provider.cast();
    }
    if pattern == UIA_ExpandCollapsePatternId && node.expanded().is_some() {
        let provider: IExpandCollapseProvider = ExpandCollapsePattern { core }.into();
        return provider.cast();
    }
    Err(Error::empty())
}

fn navigate(
    shared: &Arc<UiaShared>,
    serial: u64,
    direction: NavigateDirection,
) -> WinResult<IRawElementProviderFragment> {
    let target = {
        let state = shared.lock_win()?;
        let current = state
            .current
            .as_ref()
            .ok_or_else(element_unavailable_error)?;
        let node = current
            .nodes
            .get(&serial)
            .ok_or_else(element_unavailable_error)?;
        if direction == NavigateDirection_Parent {
            node.parent()
        } else if direction == NavigateDirection_FirstChild {
            node.children().first().copied()
        } else if direction == NavigateDirection_LastChild {
            node.children().last().copied()
        } else if direction == NavigateDirection_NextSibling
            || direction == NavigateDirection_PreviousSibling
        {
            let Some(parent) = node.parent() else {
                return Err(Error::empty());
            };
            let parent = current
                .nodes
                .get(&parent)
                .ok_or_else(element_unavailable_error)?;
            let Some(index) = parent.children().iter().position(|&child| child == serial) else {
                return Err(element_unavailable_error());
            };
            if direction == NavigateDirection_NextSibling {
                parent.children().get(index + 1).copied()
            } else {
                index
                    .checked_sub(1)
                    .and_then(|previous| parent.children().get(previous).copied())
            }
        } else {
            None
        }
    };
    match target {
        Some(serial) => fragment_provider_from_arc(shared, serial),
        None => Err(Error::empty()),
    }
}

fn bounding_rectangle(shared: &Arc<UiaShared>, serial: u64) -> WinResult<UiaRect> {
    let (_, node) = shared.current_node_win(serial)?;
    let Some(bounds) = node.bounds() else {
        return Ok(UiaRect::default());
    };
    let origin = client_origin_screen(shared.hwnd)?;
    Ok(UiaRect {
        left: f64::from(origin.x) + bounds.x(),
        top: f64::from(origin.y) + bounds.y(),
        width: bounds.width(),
        height: bounds.height(),
    })
}

fn set_focus(shared: &Arc<UiaShared>, serial: u64) -> WinResult<()> {
    let (_, node) = shared.current_node_win(serial)?;
    if !node.focusable() {
        return Err(Error::from(UIA_E_NOTSUPPORTED_HRESULT));
    }
    shared.enqueue_action(serial, WindowsAccessibilityNativeAction::Focus)
}

fn hit_test(shared: &Arc<UiaShared>, screen_x: f64, screen_y: f64) -> WinResult<Option<u64>> {
    if !screen_x.is_finite() || !screen_y.is_finite() {
        return Ok(None);
    }
    let origin = client_origin_screen(shared.hwnd)?;
    let client_x = screen_x - f64::from(origin.x);
    let client_y = screen_y - f64::from(origin.y);
    let state = shared.lock_win()?;
    let current = state
        .current
        .as_ref()
        .ok_or_else(element_unavailable_error)?;
    let mut best = None;
    let mut pending = vec![(current.root_provider_serial, 0usize)];
    while let Some((serial, depth)) = pending.pop() {
        let Some(node) = current.nodes.get(&serial) else {
            return Err(element_unavailable_error());
        };
        if node
            .bounds()
            .is_some_and(|bounds| bounds.contains(client_x, client_y))
        {
            if best.is_none_or(|(_, best_depth)| depth >= best_depth) {
                best = Some((serial, depth));
            }
            pending.extend(
                node.children()
                    .iter()
                    .rev()
                    .map(|&child| (child, depth + 1)),
            );
        }
    }
    Ok(best.map(|(serial, _)| serial))
}

fn runtime_id(serial: u64) -> WinResult<*mut SAFEARRAY> {
    let serial = i32::try_from(serial).map_err(|_| element_unavailable_error())?;
    let array = unsafe { SafeArrayCreateVector(VT_I4, 0, 2) };
    if array.is_null() {
        return Err(Error::from(E_OUTOFMEMORY));
    }
    for (index, value) in [UiaAppendRuntimeId as i32, serial].into_iter().enumerate() {
        let index = index as i32;
        let result = unsafe {
            SafeArrayPutElement(
                array,
                &index,
                (&value as *const i32).cast::<std::ffi::c_void>(),
            )
        };
        if result < 0 {
            unsafe { SafeArrayDestroy(array) };
            return Err(Error::from(HRESULT(result)));
        }
    }
    Ok(array.cast::<SAFEARRAY>())
}

fn raise_event(
    provider: IRawElementProviderSimple,
    kind: WindowsAccessibilityNativeEventKind,
    current: Option<&WindowsAccessibilityNativeNode>,
    previous: Option<&WindowsAccessibilityNativeNode>,
) -> Result<(), WindowsAccessibilityNativeError> {
    let result = unsafe {
        match kind {
            WindowsAccessibilityNativeEventKind::FocusChanged => {
                UiaRaiseAutomationEvent(&provider, UIA_AutomationFocusChangedEventId)
            }
            WindowsAccessibilityNativeEventKind::Invoked => {
                UiaRaiseAutomationEvent(&provider, UIA_Invoke_InvokedEventId)
            }
            WindowsAccessibilityNativeEventKind::NameChanged => match (previous, current) {
                (Some(previous), Some(current)) => UiaRaiseAutomationPropertyChangedEvent(
                    &provider,
                    UIA_NamePropertyId,
                    &VARIANT::from(BSTR::from(previous.name())),
                    &VARIANT::from(BSTR::from(current.name())),
                ),
                _ => Ok(()),
            },
            WindowsAccessibilityNativeEventKind::StateChanged => {
                raise_state_changes(&provider, previous, current)
            }
            WindowsAccessibilityNativeEventKind::BoundsChanged => {
                UiaRaiseAutomationEvent(&provider, UIA_LayoutInvalidatedEventId)
            }
            WindowsAccessibilityNativeEventKind::TreeChanged => UiaRaiseStructureChangedEvent(
                &provider,
                StructureChangeType_ChildrenInvalidated,
                std::ptr::null_mut(),
                0,
            ),
            WindowsAccessibilityNativeEventKind::ValueChanged => {
                return Err(native_error(
                    WindowsAccessibilityNativeErrorKind::UnsupportedPattern,
                ));
            }
        }
    };
    result.map_err(|_| native_error(WindowsAccessibilityNativeErrorKind::ComFailure))
}

unsafe fn raise_state_changes(
    provider: &IRawElementProviderSimple,
    previous: Option<&WindowsAccessibilityNativeNode>,
    current: Option<&WindowsAccessibilityNativeNode>,
) -> WinResult<()> {
    let (Some(previous), Some(current)) = (previous, current) else {
        return Ok(());
    };
    if previous.disabled() != current.disabled() {
        unsafe {
            UiaRaiseAutomationPropertyChangedEvent(
                provider,
                UIA_IsEnabledPropertyId,
                &VARIANT::from(!previous.disabled()),
                &VARIANT::from(!current.disabled()),
            )?
        };
    }
    if previous.focusable() != current.focusable() {
        unsafe {
            UiaRaiseAutomationPropertyChangedEvent(
                provider,
                UIA_IsKeyboardFocusablePropertyId,
                &VARIANT::from(previous.focusable()),
                &VARIANT::from(current.focusable()),
            )?
        };
    }
    if previous.checked() != current.checked() {
        if let (Some(previous), Some(current)) = (previous.checked(), current.checked()) {
            unsafe {
                UiaRaiseAutomationPropertyChangedEvent(
                    provider,
                    UIA_ToggleToggleStatePropertyId,
                    &VARIANT::from(toggle_state_value(previous)),
                    &VARIANT::from(toggle_state_value(current)),
                )?
            };
        }
    }
    if previous.expanded() != current.expanded() {
        if let (Some(previous), Some(current)) = (previous.expanded(), current.expanded()) {
            unsafe {
                UiaRaiseAutomationPropertyChangedEvent(
                    provider,
                    UIA_ExpandCollapseExpandCollapseStatePropertyId,
                    &VARIANT::from(expand_state_value(previous)),
                    &VARIANT::from(expand_state_value(current)),
                )?
            };
        }
    }
    Ok(())
}

fn validate_window(hwnd: NonZeroIsize) -> Result<(), WindowsAccessibilityNativeError> {
    if unsafe { IsWindow(raw_hwnd(hwnd)) } == 0 {
        return Err(native_error(
            WindowsAccessibilityNativeErrorKind::InvalidWindow,
        ));
    }
    Ok(())
}

fn client_origin_screen(hwnd: NonZeroIsize) -> WinResult<POINT> {
    let mut point = POINT { x: 0, y: 0 };
    if unsafe { ClientToScreen(raw_hwnd(hwnd), &mut point) } == 0 {
        return Err(element_unavailable_error());
    }
    Ok(point)
}

fn raw_hwnd(hwnd: NonZeroIsize) -> HWND {
    hwnd.get() as HWND
}

fn typed_hwnd(hwnd: HWND) -> TypedHwnd {
    TypedHwnd(hwnd)
}

fn element_unavailable_error() -> Error {
    Error::from(UIA_E_ELEMENTNOTAVAILABLE_HRESULT)
}

const fn control_type(role: WindowsAccessibilityNativeRole) -> i32 {
    match role {
        WindowsAccessibilityNativeRole::RootWebArea => 50_030,
        WindowsAccessibilityNativeRole::GenericContainer => 50_026,
        WindowsAccessibilityNativeRole::StaticText => 50_020,
        WindowsAccessibilityNativeRole::Button => 50_000,
        WindowsAccessibilityNativeRole::Link => 50_005,
        WindowsAccessibilityNativeRole::Heading => 50_020,
        WindowsAccessibilityNativeRole::TextField => 50_004,
        WindowsAccessibilityNativeRole::CheckBox => 50_002,
        WindowsAccessibilityNativeRole::RadioButton => 50_013,
        WindowsAccessibilityNativeRole::Image => 50_006,
        WindowsAccessibilityNativeRole::List => 50_008,
        WindowsAccessibilityNativeRole::ListItem => 50_007,
    }
}

const fn toggle_state_value(checked: bool) -> i32 {
    if checked { 1 } else { 0 }
}

const fn expand_state_value(expanded: bool) -> i32 {
    if expanded { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_runtime_serial_stays_in_i32_domain() {
        assert!(crate::accessibility::valid_provider_serial(1));
        assert!(crate::accessibility::valid_provider_serial(i32::MAX as u64));
        assert!(!crate::accessibility::valid_provider_serial(
            i32::MAX as u64 + 1
        ));
    }

    #[test]
    fn control_types_are_stable_for_supported_roles() {
        assert_eq!(control_type(WindowsAccessibilityNativeRole::Button), 50_000);
        assert_eq!(
            control_type(WindowsAccessibilityNativeRole::RootWebArea),
            50_030
        );
    }
}
