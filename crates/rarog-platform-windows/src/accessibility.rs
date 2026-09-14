use rarog_platform::{
    PlatformAccessibilityAction, PlatformAccessibilityActionRequest, PlatformAccessibilityError,
    PlatformAccessibilityErrorKind, PlatformAccessibilityEvent,
    PlatformAccessibilityEventDisposition, PlatformAccessibilityEventKind,
    PlatformAccessibilityNodeId, PlatformAccessibilityService, PlatformAccessibilitySnapshot,
    PlatformAccessibilitySnapshotReport,
};
use rarog_platform_windows_native::{
    WindowsAccessibilityNativeBridge, WindowsAccessibilityNativeErrorKind,
    WindowsAccessibilityNativeEventKind,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::num::{NonZeroU64, NonZeroUsize};
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

trait NativeAccessibilityBackend: Send + fmt::Debug {
    fn publish_event(
        &mut self,
        provider_serial: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeErrorKind>;
}

impl NativeAccessibilityBackend for WindowsAccessibilityNativeBridge {
    fn publish_event(
        &mut self,
        provider_serial: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeErrorKind> {
        WindowsAccessibilityNativeBridge::publish_event(self, provider_serial, kind)
            .map_err(|error| error.kind())
    }
}

#[derive(Debug)]
struct WindowsAccessibilityState {
    limits: WindowsAccessibilityLimits,
    current: Option<SnapshotIdentity>,
    providers_by_node: BTreeMap<PlatformAccessibilityNodeId, WindowsAccessibilityProviderId>,
    nodes_by_provider: BTreeMap<WindowsAccessibilityProviderId, PlatformAccessibilityNodeId>,
    next_provider: u64,
    pending_events: VecDeque<PlatformAccessibilityEvent>,
    action_requests: VecDeque<PlatformAccessibilityActionRequest>,
    last_native_error: Option<WindowsAccessibilityNativeErrorKind>,
    native: Box<dyn NativeAccessibilityBackend>,
}

#[derive(Debug)]
pub struct WindowsAccessibilityService {
    state: Mutex<WindowsAccessibilityState>,
}

impl WindowsAccessibilityService {
    pub fn try_new(
        limits: WindowsAccessibilityLimits,
    ) -> Result<Self, WindowsAccessibilityBridgeError> {
        if !Self::target_available() {
            return Err(WindowsAccessibilityBridgeError::UnsupportedTarget);
        }
        let native = WindowsAccessibilityNativeBridge::try_new()
            .map_err(|error| WindowsAccessibilityBridgeError::Native(error.kind()))?;
        Ok(Self::with_backend(limits, Box::new(native)))
    }

    pub fn with_default_limits() -> Result<Self, WindowsAccessibilityBridgeError> {
        Self::try_new(WindowsAccessibilityLimits::default())
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

    pub fn pending_action_count(&self) -> Result<usize, PlatformAccessibilityError> {
        Ok(self.lock_state()?.action_requests.len())
    }

    pub fn last_native_error(
        &self,
    ) -> Result<Option<WindowsAccessibilityNativeErrorKind>, PlatformAccessibilityError> {
        Ok(self.lock_state()?.last_native_error)
    }

    pub fn accept_native_action_callback(
        &self,
        provider_serial: u64,
        document_generation: u64,
        geometry_revision: u64,
        action: PlatformAccessibilityAction,
    ) -> Result<(), PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        let provider = WindowsAccessibilityProviderId(
            NonZeroU64::new(provider_serial).ok_or_else(stale_correlation)?,
        );
        let target = state
            .nodes_by_provider
            .get(&provider)
            .copied()
            .ok_or_else(stale_correlation)?;
        let current = state.current.as_ref().ok_or_else(stale_correlation)?;
        if current.document_generation != document_generation
            || current.geometry_revision != geometry_revision
            || !current.nodes.contains(&target)
        {
            return Err(stale_correlation());
        }
        if state.action_requests.len() >= state.limits.max_action_requests() {
            return Err(platform_error(
                PlatformAccessibilityErrorKind::CapacityExceeded,
            ));
        }
        state
            .action_requests
            .push_back(PlatformAccessibilityActionRequest::new(
                target,
                action,
                document_generation,
                geometry_revision,
            ));
        Ok(())
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
                action_requests: VecDeque::new(),
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
        Ok(self.lock_state()?.pending_events.front().copied())
    }
}

impl PlatformAccessibilityService for WindowsAccessibilityService {
    fn clear_snapshot(&self) -> Result<(), PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        state.current = None;
        state.providers_by_node.clear();
        state.nodes_by_provider.clear();
        state.pending_events.clear();
        state.action_requests.clear();
        state.last_native_error = None;
        Ok(())
    }

    fn replace_snapshot(
        &self,
        snapshot: &PlatformAccessibilitySnapshot,
    ) -> Result<PlatformAccessibilitySnapshotReport, PlatformAccessibilityError> {
        let mut state = self.lock_state()?;
        flush_pending(&mut state);
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

        if scope_changed {
            state.pending_events.clear();
            state.action_requests.clear();
        } else {
            let has_stale_pending = state.pending_events.iter().any(|event| {
                event.document_generation() != snapshot.document_generation()
                    || !nodes.contains(&event.target())
            });
            state.pending_events.retain(|event| {
                event.document_generation() == snapshot.document_generation()
                    && nodes.contains(&event.target())
            });
            if has_stale_pending {
                state.pending_events.clear();
                state
                    .pending_events
                    .push_back(PlatformAccessibilityEvent::new(
                        snapshot.root(),
                        PlatformAccessibilityEventKind::TreeChanged,
                        snapshot.document_generation(),
                    ));
            }
            state.action_requests.retain(|request| {
                request.document_generation() == snapshot.document_generation()
                    && request.geometry_revision() == snapshot.geometry_revision()
                    && nodes.contains(&request.target())
            });
        }

        state.providers_by_node = providers_by_node;
        state.nodes_by_provider = nodes_by_provider;
        state.next_provider = next_provider;
        state.current = Some(SnapshotIdentity {
            scope: snapshot.scope(),
            document_generation: snapshot.document_generation(),
            geometry_revision: snapshot.geometry_revision(),
            root: snapshot.root(),
            nodes,
        });
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
        let provider = state
            .providers_by_node
            .get(&event.target())
            .copied()
            .ok_or_else(stale_correlation)?;
        match state
            .native
            .publish_event(provider.serial(), native_event_kind(event.kind()))
        {
            Ok(()) => {
                state.last_native_error = None;
                Ok(PlatformAccessibilityEventDisposition::Delivered)
            }
            Err(error) => {
                state.last_native_error = Some(error);
                queue_event_conservatively(&mut state, event);
                Ok(PlatformAccessibilityEventDisposition::Queued)
            }
        }
    }

    fn next_action_request(
        &self,
    ) -> Result<Option<PlatformAccessibilityActionRequest>, PlatformAccessibilityError> {
        Ok(self.lock_state()?.action_requests.pop_front())
    }
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
        let Some(event) = state.pending_events.front().copied() else {
            return;
        };
        let Some(current) = state.current.as_ref() else {
            state.pending_events.clear();
            return;
        };
        if event.document_generation() != current.document_generation
            || !current.nodes.contains(&event.target())
        {
            state.pending_events.pop_front();
            continue;
        }
        let Some(provider) = state.providers_by_node.get(&event.target()).copied() else {
            state.pending_events.pop_front();
            continue;
        };
        match state
            .native
            .publish_event(provider.serial(), native_event_kind(event.kind()))
        {
            Ok(()) => {
                state.pending_events.pop_front();
                state.last_native_error = None;
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
) {
    if state.pending_events.len() < state.limits.max_pending_events() {
        state.pending_events.push_back(event);
        return;
    }
    let Some(current) = state.current.as_ref() else {
        state.pending_events.clear();
        return;
    };
    state.pending_events.clear();
    state
        .pending_events
        .push_back(PlatformAccessibilityEvent::new(
            current.root,
            PlatformAccessibilityEventKind::TreeChanged,
            current.document_generation,
        ));
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
        PlatformAccessibilityNode, PlatformAccessibilityRole, PlatformAccessibilityState,
    };
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct MockNativeState {
        failure: Option<WindowsAccessibilityNativeErrorKind>,
        published: Vec<(u64, WindowsAccessibilityNativeEventKind)>,
    }

    #[derive(Debug, Clone)]
    struct MockNative(Arc<Mutex<MockNativeState>>);

    impl NativeAccessibilityBackend for MockNative {
        fn publish_event(
            &mut self,
            provider_serial: u64,
            kind: WindowsAccessibilityNativeEventKind,
        ) -> Result<(), WindowsAccessibilityNativeErrorKind> {
            let mut state = self.0.lock().unwrap();
            if let Some(error) = state.failure {
                return Err(error);
            }
            state.published.push((provider_serial, kind));
            Ok(())
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
            None,
            None,
            include_child.then_some(child).into_iter().collect(),
        )];
        if include_child {
            nodes.push(PlatformAccessibilityNode::new(
                child,
                PlatformAccessibilityRole::Button,
                "save".into(),
                PlatformAccessibilityState::new(false, None, None, true),
                None,
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
        let first = snapshot(1, 1, true);
        service.replace_snapshot(&first).unwrap();
        let root_provider = service.provider_serial_for_node(id(1)).unwrap().unwrap();
        let child_provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        assert_ne!(root_provider, child_provider);

        let second = snapshot(2, 2, false);
        let report = service.replace_snapshot(&second).unwrap();
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
    fn stale_native_action_callback_is_rejected_before_engine_queueing() {
        let (service, _) = service(WindowsAccessibilityLimits::default());
        service.replace_snapshot(&snapshot(1, 1, true)).unwrap();
        let provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        service.replace_snapshot(&snapshot(2, 2, true)).unwrap();

        assert_eq!(
            service
                .accept_native_action_callback(provider, 1, 1, PlatformAccessibilityAction::Invoke,)
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::StaleCorrelation
        );
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[test]
    fn current_native_action_callback_enters_bounded_portable_queue() {
        let limits = WindowsAccessibilityLimits::try_new(8, 8, 1).unwrap();
        let (service, _) = service(limits);
        service.replace_snapshot(&snapshot(3, 4, true)).unwrap();
        let provider = service.provider_serial_for_node(id(2)).unwrap().unwrap();
        service
            .accept_native_action_callback(provider, 3, 4, PlatformAccessibilityAction::Invoke)
            .unwrap();
        let request = service.next_action_request().unwrap().unwrap();
        assert_eq!(request.target(), id(2));
        assert_eq!(request.document_generation(), 3);
        assert_eq!(request.geometry_revision(), 4);
    }

    #[test]
    fn provider_capacity_failure_is_atomic() {
        let limits = WindowsAccessibilityLimits::try_new(1, 8, 8).unwrap();
        let (service, _) = service(limits);
        assert_eq!(
            service
                .replace_snapshot(&snapshot(1, 1, true))
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::CapacityExceeded
        );
        assert_eq!(service.provider_count().unwrap(), 0);
    }

    #[test]
    fn failed_native_delivery_is_retained_and_retried_without_state_rollback() {
        let (service, native) = service(WindowsAccessibilityLimits::default());
        let snapshot = snapshot(5, 2, true);
        service.replace_snapshot(&snapshot).unwrap();
        native.lock().unwrap().failure =
            Some(WindowsAccessibilityNativeErrorKind::ProviderUnavailable);
        let event =
            PlatformAccessibilityEvent::new(id(2), PlatformAccessibilityEventKind::NameChanged, 5);
        assert_eq!(
            service.publish_event(event).unwrap(),
            PlatformAccessibilityEventDisposition::Queued
        );
        assert_eq!(service.pending_event_count().unwrap(), 1);
        assert_eq!(service.provider_count().unwrap(), 2);

        native.lock().unwrap().failure = None;
        service.replace_snapshot(&snapshot).unwrap();
        assert_eq!(service.pending_event_count().unwrap(), 0);
        assert_eq!(native.lock().unwrap().published.len(), 1);
    }

    #[test]
    fn full_retry_queue_coalesces_to_current_root_tree_change() {
        let limits = WindowsAccessibilityLimits::try_new(8, 1, 8).unwrap();
        let (service, native) = service(limits);
        service.replace_snapshot(&snapshot(7, 3, true)).unwrap();
        native.lock().unwrap().failure =
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
