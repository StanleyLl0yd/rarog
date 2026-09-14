use rarog_platform::{
    PlatformAccessibilityAction, PlatformAccessibilityActionRequest, PlatformAccessibilityError,
    PlatformAccessibilityErrorKind, PlatformAccessibilityEvent,
    PlatformAccessibilityEventDisposition, PlatformAccessibilityEventKind,
    PlatformAccessibilityNodeId, PlatformAccessibilityRole, PlatformAccessibilityService,
    PlatformAccessibilitySnapshot, PlatformAccessibilitySnapshotReport,
};
use rarog_platform_windows_native::{
    WindowsAccessibilityNativeAction, WindowsAccessibilityNativeActionRequest,
    WindowsAccessibilityNativeBridge, WindowsAccessibilityNativeErrorKind,
    WindowsAccessibilityNativeEventKind, WindowsAccessibilityNativeNode,
    WindowsAccessibilityNativeRect, WindowsAccessibilityNativeRole,
    WindowsAccessibilityNativeSnapshot,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::num::{NonZeroIsize, NonZeroU64, NonZeroUsize};
use std::sync::Mutex;

pub const DEFAULT_MAX_WINDOWS_ACCESSIBILITY_PROVIDERS: usize = 4096;
pub const DEFAULT_MAX_WINDOWS_ACCESSIBILITY_PENDING_EVENTS: usize = 1024;
pub const DEFAULT_MAX_WINDOWS_ACCESSIBILITY_ACTION_REQUESTS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsAccessibilityLimits {
    max_providers: NonZeroUsize,
    max_pending_events: NonZeroUsize,
    max_action_requests: NonZeroUsize,
}

impl WindowsAccessibilityLimits {
    pub fn try_new(
        max_providers: usize,
        max_pending_events: usize,
        max_action_requests: usize,
    ) -> Result<Self, WindowsAccessibilityBridgeError> {
        Ok(Self {
            max_providers: NonZeroUsize::new(max_providers)
                .ok_or(WindowsAccessibilityBridgeError::InvalidLimits)?,
            max_pending_events: NonZeroUsize::new(max_pending_events)
                .ok_or(WindowsAccessibilityBridgeError::InvalidLimits)?,
            max_action_requests: NonZeroUsize::new(max_action_requests)
                .ok_or(WindowsAccessibilityBridgeError::InvalidLimits)?,
        })
    }

    pub const fn max_providers(self) -> usize {
        self.max_providers.get()
    }

    pub const fn max_pending_events(self) -> usize {
        self.max_pending_events.get()
    }

    pub const fn max_action_requests(self) -> usize {
        self.max_action_requests.get()
    }
}

impl Default for WindowsAccessibilityLimits {
    fn default() -> Self {
        Self {
            max_providers: NonZeroUsize::new(DEFAULT_MAX_WINDOWS_ACCESSIBILITY_PROVIDERS)
                .expect("non-zero Windows accessibility provider limit"),
            max_pending_events: NonZeroUsize::new(DEFAULT_MAX_WINDOWS_ACCESSIBILITY_PENDING_EVENTS)
                .expect("non-zero Windows accessibility event limit"),
            max_action_requests: NonZeroUsize::new(
                DEFAULT_MAX_WINDOWS_ACCESSIBILITY_ACTION_REQUESTS,
            )
            .expect("non-zero Windows accessibility action limit"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityBridgeError {
    UnsupportedTarget,
    InvalidLimits,
    AlreadyAttached,
    Native(WindowsAccessibilityNativeErrorKind),
}

impl fmt::Display for WindowsAccessibilityBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Windows accessibility bridge error: {self:?}")
    }
}

impl std::error::Error for WindowsAccessibilityBridgeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct WindowsAccessibilityProviderId(NonZeroU64);

impl WindowsAccessibilityProviderId {
    const fn serial(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Debug)]
struct SnapshotIdentity {
    scope: u64,
    document_generation: u64,
    geometry_revision: u64,
    root: PlatformAccessibilityNodeId,
    nodes: BTreeSet<PlatformAccessibilityNodeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingNativeEvent {
    event: PlatformAccessibilityEvent,
    geometry_revision: u64,
}

trait NativeAccessibilityBackend: Send + fmt::Debug {
    fn clear_snapshot(&mut self) -> Result<(), WindowsAccessibilityNativeErrorKind>;

    fn replace_snapshot(
        &mut self,
        snapshot: &WindowsAccessibilityNativeSnapshot,
    ) -> Result<(), WindowsAccessibilityNativeErrorKind>;

    fn publish_event(
        &mut self,
        provider_serial: u64,
        document_generation: u64,
        geometry_revision: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeErrorKind>;

    fn next_action_request(
        &mut self,
    ) -> Result<Option<WindowsAccessibilityNativeActionRequest>, WindowsAccessibilityNativeErrorKind>;
}

impl NativeAccessibilityBackend for WindowsAccessibilityNativeBridge {
    fn clear_snapshot(&mut self) -> Result<(), WindowsAccessibilityNativeErrorKind> {
        WindowsAccessibilityNativeBridge::clear_snapshot(self).map_err(|error| error.kind())
    }

    fn replace_snapshot(
        &mut self,
        snapshot: &WindowsAccessibilityNativeSnapshot,
    ) -> Result<(), WindowsAccessibilityNativeErrorKind> {
        WindowsAccessibilityNativeBridge::replace_snapshot(self, snapshot)
            .map_err(|error| error.kind())
    }

    fn publish_event(
        &mut self,
        provider_serial: u64,
        document_generation: u64,
        geometry_revision: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeErrorKind> {
        WindowsAccessibilityNativeBridge::publish_event_for_snapshot(
            self,
            provider_serial,
            document_generation,
            geometry_revision,
            kind,
        )
        .map_err(|error| error.kind())
    }

    fn next_action_request(
        &mut self,
    ) -> Result<Option<WindowsAccessibilityNativeActionRequest>, WindowsAccessibilityNativeErrorKind>
    {
        WindowsAccessibilityNativeBridge::next_action_request(self).map_err(|error| error.kind())
    }
}

#[derive(Debug)]
struct WindowsAccessibilityState {
    limits: WindowsAccessibilityLimits,
    current: Option<SnapshotIdentity>,
    providers_by_node: BTreeMap<PlatformAccessibilityNodeId, WindowsAccessibilityProviderId>,
    nodes_by_provider: BTreeMap<WindowsAccessibilityProviderId, PlatformAccessibilityNodeId>,
    next_provider: u64,
    pending_events: VecDeque<PendingNativeEvent>,
    last_native_error: Option<WindowsAccessibilityNativeErrorKind>,
    native: Box<dyn NativeAccessibilityBackend>,
}

#[derive(Debug)]
pub struct WindowsAccessibilityService {
    state: Mutex<WindowsAccessibilityState>,
}

impl WindowsAccessibilityService {
    pub fn try_for_window(
        limits: WindowsAccessibilityLimits,
        hwnd: NonZeroIsize,
    ) -> Result<Self, WindowsAccessibilityBridgeError> {
        if !Self::target_available() {
            return Err(WindowsAccessibilityBridgeError::UnsupportedTarget);
        }
        let native = WindowsAccessibilityNativeBridge::try_for_window_with_action_limit(
            hwnd,
            limits.max_action_requests,
        )
        .map_err(|error| WindowsAccessibilityBridgeError::Native(error.kind()))?;
        Ok(Self::with_backend(limits, Box::new(native)))
    }

    pub fn with_default_limits_for_window(
        hwnd: NonZeroIsize,
    ) -> Result<Self, WindowsAccessibilityBridgeError> {
        Self::try_for_window(WindowsAccessibilityLimits::default(), hwnd)
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub fn limits(&self) -> Result<WindowsAccessibilityLimits, PlatformAccessibilityError> {
        Ok(self.lock_state()?.limits)
    }

    pub fn provider_count(&self) -> Result<usize, PlatformAccessibilityError> {
        Ok(self.lock_state()?.providers_by_node.len())
    }

    pub fn pending_event_count(&self) -> Result<usize, PlatformAccessibilityError> {
        Ok(self.lock_state()?.pending_events.len())
    }

    pub fn last_native_error(
        &self,
    ) -> Result<Option<WindowsAccessibilityNativeErrorKind>, PlatformAccessibilityError> {
        Ok(self.lock_state()?.last_native_error)
    }

    fn with_backend(
        limits: WindowsAccessibilityLimits,
        native: Box<dyn NativeAccessibilityBackend>,
    ) -> Self {
        Self {
            state: Mutex::new(WindowsAccessibilityState {
                limits,
                current: None,
                providers_by_node: BTreeMap::new(),
                nodes_by_provider: BTreeMap::new(),
                next_provider: 1,
                pending_events: VecDeque::new(),
                last_native_error: None,
                native,
            }),
        }
    }

    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, WindowsAccessibilityState>, PlatformAccessibilityError>
    {
        self.state
            .lock()
            .map_err(|_| platform_error(PlatformAccessibilityErrorKind::Unavailable))
    }

    #[cfg(test)]
    fn provider_serial_for_node(
        &self,
        node: PlatformAccessibilityNodeId,
    ) -> Result<Option<u64>, PlatformAccessibilityError> {
        Ok(self
            .lock_state()?
            .providers_by_node
            .get(&node)
            .map(|provider| provider.serial()))
    }

    #[cfg(test)]
    fn pending_front(
        &self,
    ) -> Result<Option<PlatformAccessibilityEvent>, PlatformAccessibilityError> {
        Ok(self
            .lock_state()?
            .pending_events
            .front()
            .map(|pending| pending.event))
    }
}

impl PlatformAccessibilityService for WindowsAccessibilityService {
    fn clear_snapshot(&self) -> Result<(), PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        let native_result = state.native.clear_snapshot();
        state.current = None;
        state.providers_by_node.clear();
        state.nodes_by_provider.clear();
        state.pending_events.clear();
        match native_result {
            Ok(()) => {
                state.last_native_error = None;
                Ok(())
            }
            Err(error) => {
                state.last_native_error = Some(error);
                Err(native_platform_error(error))
            }
        }
    }

    fn replace_snapshot(
        &self,
        snapshot: &PlatformAccessibilitySnapshot,
    ) -> Result<PlatformAccessibilitySnapshotReport, PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        if snapshot.node_count() > state.limits.max_providers() {
            return Err(platform_error(
                PlatformAccessibilityErrorKind::CapacityExceeded,
            ));
        }
        let nodes = snapshot
            .nodes()
            .iter()
            .map(|node| node.id())
            .collect::<BTreeSet<_>>();
        if nodes.len() != snapshot.node_count()
            || !nodes.contains(&snapshot.root())
            || nodes.iter().any(|node| node.scope() != snapshot.scope())
        {
            return Err(platform_error(
                PlatformAccessibilityErrorKind::InvalidSnapshot,
            ));
        }

        let scope_changed = state
            .current
            .as_ref()
            .is_some_and(|current| current.scope != snapshot.scope());
        let old_provider_count = state.providers_by_node.len();
        let mut providers_by_node = if scope_changed {
            BTreeMap::new()
        } else {
            state.providers_by_node.clone()
        };
        providers_by_node.retain(|node, _| nodes.contains(node));
        let retained_providers = providers_by_node.len();
        let mut nodes_by_provider = providers_by_node
            .iter()
            .map(|(&node, &provider)| (provider, node))
            .collect::<BTreeMap<_, _>>();
        let mut next_provider = state.next_provider;
        for &node in &nodes {
            if providers_by_node.contains_key(&node) {
                continue;
            }
            let provider = allocate_provider(&mut next_provider)?;
            providers_by_node.insert(node, provider);
            nodes_by_provider.insert(provider, node);
        }
        let created_providers = providers_by_node.len() - retained_providers;
        let retired_providers = old_provider_count.saturating_sub(retained_providers);

        let native_snapshot = project_native_snapshot(snapshot, &providers_by_node)?;
        if let Err(error) = state.native.replace_snapshot(&native_snapshot) {
            state.last_native_error = Some(error);
            return Err(native_platform_error(error));
        }

        let pending_events =
            reconcile_pending_events(&state.pending_events, snapshot, &nodes, scope_changed);
        state.providers_by_node = providers_by_node;
        state.nodes_by_provider = nodes_by_provider;
        state.next_provider = next_provider;
        state.pending_events = pending_events;
        state.current = Some(SnapshotIdentity {
            scope: snapshot.scope(),
            document_generation: snapshot.document_generation(),
            geometry_revision: snapshot.geometry_revision(),
            root: snapshot.root(),
            nodes,
        });
        state.last_native_error = None;
        flush_pending(&mut state);
        Ok(PlatformAccessibilitySnapshotReport {
            retained_providers,
            created_providers,
            retired_providers,
        })
    }

    fn publish_event(
        &self,
        event: PlatformAccessibilityEvent,
    ) -> Result<PlatformAccessibilityEventDisposition, PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        flush_pending(&mut state);
        let current = state.current.as_ref().ok_or_else(stale_correlation)?;
        if event.target().scope() != current.scope
            || event.document_generation() != current.document_generation
            || !current.nodes.contains(&event.target())
        {
            return Err(stale_correlation());
        }
        let geometry_revision = current.geometry_revision;
        let provider = state
            .providers_by_node
            .get(&event.target())
            .copied()
            .ok_or_else(stale_correlation)?;
        match state.native.publish_event(
            provider.serial(),
            event.document_generation(),
            geometry_revision,
            native_event_kind(event.kind()),
        ) {
            Ok(()) => {
                state.last_native_error = None;
                Ok(PlatformAccessibilityEventDisposition::Delivered)
            }
            Err(error) if permanent_native_error(error) => {
                state.last_native_error = Some(error);
                Err(native_platform_error(error))
            }
            Err(error) => {
                state.last_native_error = Some(error);
                queue_event_conservatively(&mut state, event, geometry_revision);
                Ok(PlatformAccessibilityEventDisposition::Queued)
            }
        }
    }

    fn next_action_request(
        &self,
    ) -> Result<Option<PlatformAccessibilityActionRequest>, PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        for _ in 0..state.limits.max_action_requests() {
            let request = match state.native.next_action_request() {
                Ok(Some(request)) => request,
                Ok(None) => {
                    state.last_native_error = None;
                    return Ok(None);
                }
                Err(error) => {
                    state.last_native_error = Some(error);
                    return Err(native_platform_error(error));
                }
            };
            let Some(current) = state.current.as_ref() else {
                continue;
            };
            if request.document_generation() != current.document_generation
                || request.geometry_revision() != current.geometry_revision
            {
                continue;
            }
            let Some(provider) =
                NonZeroU64::new(request.provider_serial()).map(WindowsAccessibilityProviderId)
            else {
                continue;
            };
            let Some(target) = state.nodes_by_provider.get(&provider).copied() else {
                continue;
            };
            if target.scope() != current.scope || !current.nodes.contains(&target) {
                continue;
            }
            state.last_native_error = None;
            return Ok(Some(PlatformAccessibilityActionRequest::new(
                target,
                platform_action(request.action()),
                request.document_generation(),
                request.geometry_revision(),
            )));
        }
        Ok(None)
    }
}

fn project_native_snapshot(
    snapshot: &PlatformAccessibilitySnapshot,
    providers_by_node: &BTreeMap<PlatformAccessibilityNodeId, WindowsAccessibilityProviderId>,
) -> Result<WindowsAccessibilityNativeSnapshot, PlatformAccessibilityError> {
    let root_provider = providers_by_node
        .get(&snapshot.root())
        .copied()
        .ok_or_else(stale_correlation)?;
    let mut native_nodes = Vec::with_capacity(snapshot.node_count());
    for node in snapshot.nodes() {
        let provider = providers_by_node
            .get(&node.id())
            .copied()
            .ok_or_else(stale_correlation)?;
        let parent = match node.parent() {
            Some(parent) => Some(
                providers_by_node
                    .get(&parent)
                    .copied()
                    .ok_or_else(stale_correlation)?
                    .serial(),
            ),
            None => None,
        };
        let children = node
            .children()
            .iter()
            .map(|child| {
                providers_by_node
                    .get(child)
                    .copied()
                    .map(WindowsAccessibilityProviderId::serial)
                    .ok_or_else(stale_correlation)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bounds = node
            .bounds()
            .map(|bounds| {
                WindowsAccessibilityNativeRect::try_new(
                    f64::from(bounds.x),
                    f64::from(bounds.y),
                    f64::from(bounds.width),
                    f64::from(bounds.height),
                )
                .map_err(|error| native_platform_error(error.kind()))
            })
            .transpose()?;
        let state = node.state();
        native_nodes.push(WindowsAccessibilityNativeNode::new(
            provider.serial(),
            native_role(node.role()),
            node.name().to_owned(),
            state.disabled(),
            state.checked(),
            state.expanded(),
            state.focusable(),
            bounds,
            parent,
            children,
        ));
    }
    WindowsAccessibilityNativeSnapshot::try_new(
        snapshot.document_generation(),
        snapshot.geometry_revision(),
        root_provider.serial(),
        native_nodes,
    )
    .map_err(|error| native_platform_error(error.kind()))
}

fn reconcile_pending_events(
    pending: &VecDeque<PendingNativeEvent>,
    snapshot: &PlatformAccessibilitySnapshot,
    nodes: &BTreeSet<PlatformAccessibilityNodeId>,
    scope_changed: bool,
) -> VecDeque<PendingNativeEvent> {
    if scope_changed || pending.is_empty() {
        return VecDeque::new();
    }
    let stale = pending.iter().any(|pending| {
        pending.event.document_generation() != snapshot.document_generation()
            || pending.geometry_revision != snapshot.geometry_revision()
            || !nodes.contains(&pending.event.target())
    });
    if stale {
        return VecDeque::from([PendingNativeEvent {
            event: PlatformAccessibilityEvent::new(
                snapshot.root(),
                PlatformAccessibilityEventKind::TreeChanged,
                snapshot.document_generation(),
            ),
            geometry_revision: snapshot.geometry_revision(),
        }]);
    }
    pending.clone()
}

fn allocate_provider(
    next_provider: &mut u64,
) -> Result<WindowsAccessibilityProviderId, PlatformAccessibilityError> {
    if *next_provider == 0 || *next_provider > i32::MAX as u64 {
        return Err(platform_error(
            PlatformAccessibilityErrorKind::CapacityExceeded,
        ));
    }
    let provider = WindowsAccessibilityProviderId(
        NonZeroU64::new(*next_provider)
            .ok_or_else(|| platform_error(PlatformAccessibilityErrorKind::CapacityExceeded))?,
    );
    *next_provider = next_provider
        .checked_add(1)
        .ok_or_else(|| platform_error(PlatformAccessibilityErrorKind::CapacityExceeded))?;
    Ok(provider)
}

fn flush_pending(state: &mut WindowsAccessibilityState) {
    loop {
        let Some(pending) = state.pending_events.front().copied() else {
            return;
        };
        let Some(current) = state.current.as_ref() else {
            state.pending_events.clear();
            return;
        };
        let event = pending.event;
        if event.document_generation() != current.document_generation
            || pending.geometry_revision != current.geometry_revision
            || !current.nodes.contains(&event.target())
        {
            state.pending_events.pop_front();
            continue;
        }
        let Some(provider) = state.providers_by_node.get(&event.target()).copied() else {
            state.pending_events.pop_front();
            continue;
        };
        match state.native.publish_event(
            provider.serial(),
            event.document_generation(),
            pending.geometry_revision,
            native_event_kind(event.kind()),
        ) {
            Ok(()) => {
                state.pending_events.pop_front();
                state.last_native_error = None;
            }
            Err(error) if permanent_native_error(error) => {
                state.pending_events.pop_front();
                state.last_native_error = Some(error);
            }
            Err(error) => {
                state.last_native_error = Some(error);
                return;
            }
        }
    }
}

fn queue_event_conservatively(
    state: &mut WindowsAccessibilityState,
    event: PlatformAccessibilityEvent,
    geometry_revision: u64,
) {
    if state.pending_events.len() < state.limits.max_pending_events() {
        state.pending_events.push_back(PendingNativeEvent {
            event,
            geometry_revision,
        });
        return;
    }
    let Some(current) = state.current.as_ref() else {
        state.pending_events.clear();
        return;
    };
    state.pending_events.clear();
    state.pending_events.push_back(PendingNativeEvent {
        event: PlatformAccessibilityEvent::new(
            current.root,
            PlatformAccessibilityEventKind::TreeChanged,
            current.document_generation,
        ),
        geometry_revision: current.geometry_revision,
    });
}

const fn native_role(role: PlatformAccessibilityRole) -> WindowsAccessibilityNativeRole {
    match role {
        PlatformAccessibilityRole::RootWebArea => WindowsAccessibilityNativeRole::RootWebArea,
        PlatformAccessibilityRole::GenericContainer => {
            WindowsAccessibilityNativeRole::GenericContainer
        }
        PlatformAccessibilityRole::StaticText => WindowsAccessibilityNativeRole::StaticText,
        PlatformAccessibilityRole::Button => WindowsAccessibilityNativeRole::Button,
        PlatformAccessibilityRole::Link => WindowsAccessibilityNativeRole::Link,
        PlatformAccessibilityRole::Heading => WindowsAccessibilityNativeRole::Heading,
        PlatformAccessibilityRole::TextField => WindowsAccessibilityNativeRole::TextField,
        PlatformAccessibilityRole::CheckBox => WindowsAccessibilityNativeRole::CheckBox,
        PlatformAccessibilityRole::RadioButton => WindowsAccessibilityNativeRole::RadioButton,
        PlatformAccessibilityRole::Image => WindowsAccessibilityNativeRole::Image,
        PlatformAccessibilityRole::List => WindowsAccessibilityNativeRole::List,
        PlatformAccessibilityRole::ListItem => WindowsAccessibilityNativeRole::ListItem,
    }
}

const fn native_event_kind(
    kind: PlatformAccessibilityEventKind,
) -> WindowsAccessibilityNativeEventKind {
    match kind {
        PlatformAccessibilityEventKind::FocusChanged => {
            WindowsAccessibilityNativeEventKind::FocusChanged
        }
        PlatformAccessibilityEventKind::Invoked => WindowsAccessibilityNativeEventKind::Invoked,
        PlatformAccessibilityEventKind::StateChanged => {
            WindowsAccessibilityNativeEventKind::StateChanged
        }
        PlatformAccessibilityEventKind::NameChanged => {
            WindowsAccessibilityNativeEventKind::NameChanged
        }
        PlatformAccessibilityEventKind::ValueChanged => {
            WindowsAccessibilityNativeEventKind::ValueChanged
        }
        PlatformAccessibilityEventKind::BoundsChanged => {
            WindowsAccessibilityNativeEventKind::BoundsChanged
        }
        PlatformAccessibilityEventKind::TreeChanged => {
            WindowsAccessibilityNativeEventKind::TreeChanged
        }
    }
}

const fn platform_action(action: WindowsAccessibilityNativeAction) -> PlatformAccessibilityAction {
    match action {
        WindowsAccessibilityNativeAction::Focus => PlatformAccessibilityAction::Focus,
        WindowsAccessibilityNativeAction::Invoke => PlatformAccessibilityAction::Invoke,
        WindowsAccessibilityNativeAction::Toggle => PlatformAccessibilityAction::Toggle,
        WindowsAccessibilityNativeAction::SetExpanded(expanded) => {
            PlatformAccessibilityAction::SetExpanded(expanded)
        }
    }
}

const fn permanent_native_error(error: WindowsAccessibilityNativeErrorKind) -> bool {
    matches!(
        error,
        WindowsAccessibilityNativeErrorKind::UnsupportedPattern
            | WindowsAccessibilityNativeErrorKind::InvalidSnapshot
            | WindowsAccessibilityNativeErrorKind::CapacityExceeded
            | WindowsAccessibilityNativeErrorKind::UnsupportedTarget
            | WindowsAccessibilityNativeErrorKind::InvalidWindow
    )
}

const fn native_platform_error(
    error: WindowsAccessibilityNativeErrorKind,
) -> PlatformAccessibilityError {
    platform_error(match error {
        WindowsAccessibilityNativeErrorKind::UnsupportedTarget => {
            PlatformAccessibilityErrorKind::UnsupportedTarget
        }
        WindowsAccessibilityNativeErrorKind::InvalidWindow => {
            PlatformAccessibilityErrorKind::NativeUnavailable
        }
        WindowsAccessibilityNativeErrorKind::InvalidSnapshot => {
            PlatformAccessibilityErrorKind::InvalidSnapshot
        }
        WindowsAccessibilityNativeErrorKind::CapacityExceeded => {
            PlatformAccessibilityErrorKind::CapacityExceeded
        }
        WindowsAccessibilityNativeErrorKind::ProviderUnavailable => {
            PlatformAccessibilityErrorKind::StaleCorrelation
        }
        WindowsAccessibilityNativeErrorKind::ComFailure => {
            PlatformAccessibilityErrorKind::NativeFailure
        }
        WindowsAccessibilityNativeErrorKind::UnsupportedPattern => {
            PlatformAccessibilityErrorKind::UnsupportedPattern
        }
    })
}

const fn platform_error(kind: PlatformAccessibilityErrorKind) -> PlatformAccessibilityError {
    PlatformAccessibilityError::new(kind)
}

const fn stale_correlation() -> PlatformAccessibilityError {
    platform_error(PlatformAccessibilityErrorKind::StaleCorrelation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_platform::{
        PlatformAccessibilityNode, PlatformAccessibilityRect, PlatformAccessibilityState,
    };
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct MockNativeState {
        replace_failure: Option<WindowsAccessibilityNativeErrorKind>,
        event_failure: Option<WindowsAccessibilityNativeErrorKind>,
        action_failure: Option<WindowsAccessibilityNativeErrorKind>,
        clear_failure: Option<WindowsAccessibilityNativeErrorKind>,
        clear_count: usize,
        snapshots: Vec<WindowsAccessibilityNativeSnapshot>,
        published: Vec<(u64, u64, u64, WindowsAccessibilityNativeEventKind)>,
        actions: VecDeque<WindowsAccessibilityNativeActionRequest>,
    }

    #[derive(Debug, Clone)]
    struct MockNative(Arc<Mutex<MockNativeState>>);

    impl NativeAccessibilityBackend for MockNative {
        fn clear_snapshot(&mut self) -> Result<(), WindowsAccessibilityNativeErrorKind> {
            let mut state = self.0.lock().unwrap();
            state.clear_count += 1;
            if let Some(error) = state.clear_failure {
                return Err(error);
            }
            Ok(())
        }

        fn replace_snapshot(
            &mut self,
            snapshot: &WindowsAccessibilityNativeSnapshot,
        ) -> Result<(), WindowsAccessibilityNativeErrorKind> {
            let mut state = self.0.lock().unwrap();
            if let Some(error) = state.replace_failure {
                return Err(error);
            }
            state.snapshots.push(snapshot.clone());
            Ok(())
        }

        fn publish_event(
            &mut self,
            provider_serial: u64,
            document_generation: u64,
            geometry_revision: u64,
            kind: WindowsAccessibilityNativeEventKind,
        ) -> Result<(), WindowsAccessibilityNativeErrorKind> {
            let mut state = self.0.lock().unwrap();
            if let Some(error) = state.event_failure {
                return Err(error);
            }
            state.published.push((
                provider_serial,
                document_generation,
                geometry_revision,
                kind,
            ));
            Ok(())
        }

        fn next_action_request(
            &mut self,
        ) -> Result<
            Option<WindowsAccessibilityNativeActionRequest>,
            WindowsAccessibilityNativeErrorKind,
        > {
            let mut state = self.0.lock().unwrap();
            if let Some(error) = state.action_failure {
                return Err(error);
            }
            Ok(state.actions.pop_front())
        }
    }

    fn id(serial: u64) -> PlatformAccessibilityNodeId {
        PlatformAccessibilityNodeId::try_new(11, serial).unwrap()
    }

    fn snapshot(
        document_generation: u64,
        geometry_revision: u64,
        include_child: bool,
    ) -> PlatformAccessibilitySnapshot {
        let root = id(1);
        let child = id(2);
        let mut nodes = vec![PlatformAccessibilityNode::new(
            root,
            PlatformAccessibilityRole::RootWebArea,
            "root".into(),
            PlatformAccessibilityState::default(),
            Some(PlatformAccessibilityRect::new(0.0, 0.0, 640.0, 480.0)),
            None,
            include_child.then_some(child).into_iter().collect(),
        )];
        if include_child {
            nodes.push(PlatformAccessibilityNode::new(
                child,
                PlatformAccessibilityRole::Button,
                "save".into(),
                PlatformAccessibilityState::new(false, None, None, true),
                Some(PlatformAccessibilityRect::new(10.0, 20.0, 100.0, 30.0)),
                Some(root),
                Vec::new(),
            ));
        }
        PlatformAccessibilitySnapshot::try_new(document_generation, geometry_revision, root, nodes)
            .unwrap()
    }

    fn service(
        limits: WindowsAccessibilityLimits,
    ) -> (WindowsAccessibilityService, Arc<Mutex<MockNativeState>>) {
        let state = Arc::new(Mutex::new(MockNativeState::default()));
        (
            WindowsAccessibilityService::with_backend(
                limits,
                Box::new(MockNative(Arc::clone(&state))),
            ),
            state,
        )
    }

    #[test]
    fn provider_correlations_are_stable_and_absent_nodes_retire() {
        let (service, _) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(1, 1, true)).unwrap();
        let root_provider = service.provider_serial_for_node(id(1)).unwrap().unwrap();
        let child_provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        assert_ne!(root_provider, child_provider);

        let report = service.replace_snapshot(&snapshot(2, 2, false)).unwrap();
        assert_eq!(report.retained_providers, 1);
        assert_eq!(report.retired_providers, 1);
        assert_eq!(service.provider_count().unwrap(), 1);
        assert_eq!(
            service.provider_serial_for_node(id(1)).unwrap(),
            Some(root_provider)
        );
        assert_eq!(service.provider_serial_for_node(id(2)).unwrap(), None);
    }

    #[test]
    fn snapshot_projection_reaches_native_with_exact_identity_and_geometry() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(3, 4, true)).unwrap();
        let state = native.lock().unwrap();
        let projected = state.snapshots.last().unwrap();
        assert_eq!(projected.document_generation(), 3);
        assert_eq!(projected.geometry_revision(), 4);
        assert_eq!(projected.nodes().len(), 2);
        let child = projected
            .nodes()
            .iter()
            .find(|node| node.name() == "save")
            .unwrap();
        assert_eq!(child.role(), WindowsAccessibilityNativeRole::Button);
        assert!(child.focusable());
        assert_eq!(child.bounds().unwrap().x(), 10.0);
    }

    #[test]
    fn failed_native_replacement_preserves_wrapper_correlations_atomically() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(1, 1, true)).unwrap();
        let child_provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        native.lock().unwrap().replace_failure =
            Some(WindowsAccessibilityNativeErrorKind::ComFailure);

        assert_eq!(
            service
                .replace_snapshot(&snapshot(2, 2, false))
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::NativeFailure
        );
        assert_eq!(service.provider_count().unwrap(), 2);
        assert_eq!(
            service.provider_serial_for_node(id(2)).unwrap(),
            Some(child_provider)
        );
    }

    #[test]
    fn clear_snapshot_retires_correlations_and_clears_native_snapshot() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(1, 1, true)).unwrap();
        service.clear_snapshot().unwrap();
        assert_eq!(service.provider_count().unwrap(), 0);
        assert_eq!(service.pending_event_count().unwrap(), 0);
        assert_eq!(native.lock().unwrap().clear_count, 1);
    }

    #[test]
    fn stale_native_actions_are_dropped_before_engine_queueing() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(1, 1, true)).unwrap();
        let provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        service.replace_snapshot(&snapshot(2, 2, true)).unwrap();
        native
            .lock()
            .unwrap()
            .actions
            .push_back(WindowsAccessibilityNativeActionRequest::new(
                provider,
                1,
                1,
                WindowsAccessibilityNativeAction::Invoke,
            ));

        assert_eq!(service.next_action_request().unwrap(), None);
    }

    #[test]
    fn current_native_action_is_projected_to_portable_request() {
        let limits = WindowsAccessibilityLimits::try_new(8, 8, 1).unwrap();
        let (service, native) = service(limits);
        service.replace_snapshot(&snapshot(3, 4, true)).unwrap();
        let provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        native
            .lock()
            .unwrap()
            .actions
            .push_back(WindowsAccessibilityNativeActionRequest::new(
                provider,
                3,
                4,
                WindowsAccessibilityNativeAction::Invoke,
            ));
        let request = service.next_action_request().unwrap().unwrap();
        assert_eq!(request.target(), id(2));
        assert_eq!(request.action(), PlatformAccessibilityAction::Invoke);
        assert_eq!(request.document_generation(), 3);
        assert_eq!(request.geometry_revision(), 4);
    }

    #[test]
    fn provider_capacity_failure_is_atomic() {
        let limits = WindowsAccessibilityLimits::try_new(1, 8, 8).unwrap();
        let (service, native) = service(limits);
        assert_eq!(
            service
                .replace_snapshot(&snapshot(1, 1, true))
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::CapacityExceeded
        );
        assert_eq!(service.provider_count().unwrap(), 0);
        assert!(native.lock().unwrap().snapshots.is_empty());
    }

    #[test]
    fn failed_native_delivery_is_retained_and_retried_with_exact_snapshot_identity() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        let current = snapshot(5, 2, true);
        service.replace_snapshot(&current).unwrap();
        native.lock().unwrap().event_failure =
            Some(WindowsAccessibilityNativeErrorKind::ProviderUnavailable);
        let event =
            PlatformAccessibilityEvent::new(id(2), PlatformAccessibilityEventKind::NameChanged, 5);
        assert_eq!(
            service.publish_event(event).unwrap(),
            PlatformAccessibilityEventDisposition::Queued
        );
        assert_eq!(service.pending_event_count().unwrap(), 1);

        native.lock().unwrap().event_failure = None;
        service.replace_snapshot(&current).unwrap();
        assert_eq!(service.pending_event_count().unwrap(), 0);
        let published = native.lock().unwrap().published.clone();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].1, 5);
        assert_eq!(published[0].2, 2);
    }

    #[test]
    fn geometry_change_coalesces_stale_retry_to_current_root_tree_change() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(7, 3, true)).unwrap();
        native.lock().unwrap().event_failure =
            Some(WindowsAccessibilityNativeErrorKind::ProviderUnavailable);
        service
            .publish_event(PlatformAccessibilityEvent::new(
                id(2),
                PlatformAccessibilityEventKind::BoundsChanged,
                7,
            ))
            .unwrap();
        native.lock().unwrap().event_failure = None;
        service.replace_snapshot(&snapshot(7, 4, true)).unwrap();
        let published = native.lock().unwrap().published.clone();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].1, 7);
        assert_eq!(published[0].2, 4);
        assert_eq!(
            published[0].3,
            WindowsAccessibilityNativeEventKind::TreeChanged
        );
    }

    #[test]
    fn full_retry_queue_coalesces_to_current_root_tree_change() {
        let limits = WindowsAccessibilityLimits::try_new(8, 1, 8).unwrap();
        let (service, native) = service(limits);
        service.replace_snapshot(&snapshot(7, 3, true)).unwrap();
        native.lock().unwrap().event_failure =
            Some(WindowsAccessibilityNativeErrorKind::ProviderUnavailable);
        for kind in [
            PlatformAccessibilityEventKind::NameChanged,
            PlatformAccessibilityEventKind::BoundsChanged,
        ] {
            assert_eq!(
                service
                    .publish_event(PlatformAccessibilityEvent::new(id(2), kind, 7))
                    .unwrap(),
                PlatformAccessibilityEventDisposition::Queued
            );
        }
        assert_eq!(service.pending_event_count().unwrap(), 1);
        let event = service.pending_front().unwrap().unwrap();
        assert_eq!(event.target(), id(1));
        assert_eq!(event.kind(), PlatformAccessibilityEventKind::TreeChanged);
    }
}
