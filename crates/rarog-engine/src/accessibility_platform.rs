use crate::RenderSession;
use rarog_accessibility::{
    AccessibilityAction, AccessibilityActionError, AccessibilityActionExecutor,
    AccessibilityActionResult, AccessibilityEvent, AccessibilityEventKind, AccessibilityNodeId,
    AccessibilityRole, AccessibilitySnapshot,
};
use rarog_platform::{
    PlatformAccessibilityAction, PlatformAccessibilityError, PlatformAccessibilityEvent,
    PlatformAccessibilityEventDisposition, PlatformAccessibilityEventKind,
    PlatformAccessibilityNode, PlatformAccessibilityNodeId, PlatformAccessibilityRect,
    PlatformAccessibilityRole, PlatformAccessibilityService, PlatformAccessibilitySnapshot,
    PlatformAccessibilityState,
};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformAccessibilitySyncReport {
    pub snapshot_current: bool,
    pub events_delivered: usize,
    pub events_queued: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAccessibilityBridgeError {
    Platform(PlatformAccessibilityError),
    StaleRequest,
    TargetNotPresent,
    Action(AccessibilityActionError),
}

impl fmt::Display for PlatformAccessibilityBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "platform accessibility bridge error: {self:?}")
    }
}

impl std::error::Error for PlatformAccessibilityBridgeError {}

impl From<PlatformAccessibilityError> for PlatformAccessibilityBridgeError {
    fn from(error: PlatformAccessibilityError) -> Self {
        Self::Platform(error)
    }
}

impl From<AccessibilityActionError> for PlatformAccessibilityBridgeError {
    fn from(error: AccessibilityActionError) -> Self {
        Self::Action(error)
    }
}

impl RenderSession {
    pub fn sync_platform_accessibility(
        &mut self,
        service: &dyn PlatformAccessibilityService,
    ) -> Result<PlatformAccessibilitySyncReport, PlatformAccessibilityBridgeError> {
        let Some(snapshot) = self.accessibility_snapshot() else {
            service.clear_snapshot()?;
            return Ok(PlatformAccessibilitySyncReport::default());
        };
        let projection = project_snapshot(snapshot)?;
        service.replace_snapshot(&projection)?;

        let mut report = PlatformAccessibilitySyncReport {
            snapshot_current: true,
            ..PlatformAccessibilitySyncReport::default()
        };
        while let Some(event) = self.next_accessibility_event() {
            match service.publish_event(project_event(event)?)? {
                PlatformAccessibilityEventDisposition::Delivered => {
                    report.events_delivered += 1;
                }
                PlatformAccessibilityEventDisposition::Queued => {
                    report.events_queued += 1;
                }
            }
        }
        Ok(report)
    }

    pub fn perform_next_platform_accessibility_action<E: AccessibilityActionExecutor>(
        &mut self,
        service: &dyn PlatformAccessibilityService,
        executor: &mut E,
    ) -> Result<Option<AccessibilityActionResult>, PlatformAccessibilityBridgeError> {
        let Some(request) = service.next_action_request()? else {
            return Ok(None);
        };
        let snapshot = self
            .accessibility_snapshot()
            .ok_or(PlatformAccessibilityBridgeError::StaleRequest)?;
        if snapshot.document_generation() != request.document_generation()
            || snapshot.geometry_revision() != request.geometry_revision()
            || snapshot.tree().scope() != request.target().scope()
        {
            return Err(PlatformAccessibilityBridgeError::StaleRequest);
        }
        let target = find_target(snapshot, request.target())
            .ok_or(PlatformAccessibilityBridgeError::TargetNotPresent)?;
        self.perform_accessibility_action(executor, target, map_action(request.action()))
            .map(Some)
            .map_err(Into::into)
    }
}

fn project_snapshot(
    snapshot: &AccessibilitySnapshot,
) -> Result<PlatformAccessibilitySnapshot, PlatformAccessibilityError> {
    let tree = snapshot.tree();
    let root = project_id(tree.root())?;
    let mut stack = vec![tree.root()];
    let mut visited = BTreeSet::new();
    let mut nodes = Vec::with_capacity(tree.node_count());

    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        let node = tree.node(id).ok_or_else(invalid_projection)?;
        let children = node
            .children()
            .iter()
            .copied()
            .map(project_id)
            .collect::<Result<Vec<_>, _>>()?;
        stack.extend(node.children().iter().rev().copied());
        nodes.push(PlatformAccessibilityNode::new(
            project_id(node.id())?,
            project_role(node.role()),
            node.name().to_owned(),
            PlatformAccessibilityState::new(
                node.state().disabled(),
                node.state().checked(),
                node.state().expanded(),
                node.state().focusable(),
            ),
            node.bounds().map(|bounds| {
                PlatformAccessibilityRect::new(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height,
                )
            }),
            node.parent().map(project_id).transpose()?,
            children,
        ));
        if visited.len() > tree.node_count() {
            return Err(invalid_projection());
        }
    }
    if visited.len() != tree.node_count() {
        return Err(invalid_projection());
    }

    PlatformAccessibilitySnapshot::try_new(
        snapshot.document_generation(),
        snapshot.geometry_revision(),
        root,
        nodes,
    )
}

fn project_event(
    event: AccessibilityEvent,
) -> Result<PlatformAccessibilityEvent, PlatformAccessibilityError> {
    Ok(PlatformAccessibilityEvent::new(
        project_id(event.target())?,
        match event.kind() {
            AccessibilityEventKind::FocusChanged => PlatformAccessibilityEventKind::FocusChanged,
            AccessibilityEventKind::Invoked => PlatformAccessibilityEventKind::Invoked,
            AccessibilityEventKind::StateChanged => PlatformAccessibilityEventKind::StateChanged,
            AccessibilityEventKind::NameChanged => PlatformAccessibilityEventKind::NameChanged,
            AccessibilityEventKind::ValueChanged => PlatformAccessibilityEventKind::ValueChanged,
            AccessibilityEventKind::BoundsChanged => PlatformAccessibilityEventKind::BoundsChanged,
            AccessibilityEventKind::TreeChanged => PlatformAccessibilityEventKind::TreeChanged,
        },
        event.document_generation(),
    ))
}

fn project_id(
    id: AccessibilityNodeId,
) -> Result<PlatformAccessibilityNodeId, PlatformAccessibilityError> {
    PlatformAccessibilityNodeId::try_new(id.scope(), id.serial())
}

const fn project_role(role: AccessibilityRole) -> PlatformAccessibilityRole {
    match role {
        AccessibilityRole::RootWebArea => PlatformAccessibilityRole::RootWebArea,
        AccessibilityRole::GenericContainer => PlatformAccessibilityRole::GenericContainer,
        AccessibilityRole::StaticText => PlatformAccessibilityRole::StaticText,
        AccessibilityRole::Button => PlatformAccessibilityRole::Button,
        AccessibilityRole::Link => PlatformAccessibilityRole::Link,
        AccessibilityRole::Heading => PlatformAccessibilityRole::Heading,
        AccessibilityRole::TextField => PlatformAccessibilityRole::TextField,
        AccessibilityRole::CheckBox => PlatformAccessibilityRole::CheckBox,
        AccessibilityRole::RadioButton => PlatformAccessibilityRole::RadioButton,
        AccessibilityRole::Image => PlatformAccessibilityRole::Image,
        AccessibilityRole::List => PlatformAccessibilityRole::List,
        AccessibilityRole::ListItem => PlatformAccessibilityRole::ListItem,
    }
}

const fn map_action(action: PlatformAccessibilityAction) -> AccessibilityAction {
    match action {
        PlatformAccessibilityAction::Focus => AccessibilityAction::Focus,
        PlatformAccessibilityAction::Invoke => AccessibilityAction::Invoke,
        PlatformAccessibilityAction::Toggle => AccessibilityAction::Toggle,
        PlatformAccessibilityAction::SetExpanded(expanded) => {
            AccessibilityAction::SetExpanded(expanded)
        }
    }
}

fn find_target(
    snapshot: &AccessibilitySnapshot,
    target: PlatformAccessibilityNodeId,
) -> Option<AccessibilityNodeId> {
    let tree = snapshot.tree();
    if tree.scope() != target.scope() {
        return None;
    }
    let mut stack = vec![tree.root()];
    let mut visited = 0usize;
    while let Some(id) = stack.pop() {
        visited = visited.checked_add(1)?;
        if visited > tree.node_count() {
            return None;
        }
        if id.serial() == target.serial() {
            return Some(id);
        }
        let node = tree.node(id)?;
        stack.extend(node.children().iter().rev().copied());
    }
    None
}

fn invalid_projection() -> PlatformAccessibilityError {
    PlatformAccessibilityError::new(rarog_platform::PlatformAccessibilityErrorKind::InvalidSnapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RenderOptions;
    use rarog_accessibility::{
        AccessibilityActionCommand, AccessibilityActionTarget, AccessibilityExecutorErrorKind,
    };
    use rarog_platform::{
        PlatformAccessibilityActionRequest, PlatformAccessibilitySnapshotReport,
    };
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct RecordingService {
        snapshot: Mutex<Option<PlatformAccessibilitySnapshot>>,
        events: Mutex<Vec<PlatformAccessibilityEvent>>,
        actions: Mutex<Vec<PlatformAccessibilityActionRequest>>,
    }

    impl PlatformAccessibilityService for RecordingService {
        fn clear_snapshot(&self) -> Result<(), PlatformAccessibilityError> {
            *self.snapshot.lock().unwrap() = None;
            Ok(())
        }

        fn replace_snapshot(
            &self,
            snapshot: &PlatformAccessibilitySnapshot,
        ) -> Result<PlatformAccessibilitySnapshotReport, PlatformAccessibilityError> {
            *self.snapshot.lock().unwrap() = Some(snapshot.clone());
            Ok(PlatformAccessibilitySnapshotReport::default())
        }

        fn publish_event(
            &self,
            event: PlatformAccessibilityEvent,
        ) -> Result<PlatformAccessibilityEventDisposition, PlatformAccessibilityError> {
            self.events.lock().unwrap().push(event);
            Ok(PlatformAccessibilityEventDisposition::Delivered)
        }

        fn next_action_request(
            &self,
        ) -> Result<Option<PlatformAccessibilityActionRequest>, PlatformAccessibilityError> {
            Ok(self.actions.lock().unwrap().pop())
        }
    }

    #[derive(Debug, Default)]
    struct InvokeExecutor;

    impl AccessibilityActionExecutor for InvokeExecutor {
        fn execute(
            &mut self,
            _target: &mut AccessibilityActionTarget,
            command: AccessibilityActionCommand,
        ) -> Result<(), AccessibilityExecutorErrorKind> {
            match command {
                AccessibilityActionCommand::Invoke => Ok(()),
                _ => Err(AccessibilityExecutorErrorKind::Unavailable),
            }
        }
    }

    #[test]
    fn sync_projects_current_snapshot_and_committed_events() {
        let mut session =
            RenderSession::new("<button aria-label='Save'>Save</button>", RenderOptions::default())
                .unwrap();
        let service = RecordingService::default();
        let first = session.sync_platform_accessibility(&service).unwrap();
        assert!(first.snapshot_current);
        let projected = service.snapshot.lock().unwrap().clone().unwrap();
        assert_eq!(
            projected.document_generation(),
            session.accessibility_snapshot().unwrap().document_generation()
        );
        assert!(projected.node_count() >= 2);

        let button = projected
            .nodes()
            .iter()
            .find(|node| node.role() == PlatformAccessibilityRole::Button)
            .unwrap()
            .id();
        let source = session
            .accessibility_snapshot()
            .unwrap()
            .tree()
            .node_count();
        assert!(source >= 2);

        let dom_button = session
            .document()
            .root();
        assert!(session.document().node(dom_button).is_some());
        let _ = button;
    }

    #[test]
    fn stale_platform_action_is_rejected_before_dom_mutation() {
        let mut session =
            RenderSession::new("<button>Save</button>", RenderOptions::default()).unwrap();
        let service = RecordingService::default();
        session.sync_platform_accessibility(&service).unwrap();
        let snapshot = service.snapshot.lock().unwrap().clone().unwrap();
        let button = snapshot
            .nodes()
            .iter()
            .find(|node| node.role() == PlatformAccessibilityRole::Button)
            .unwrap()
            .id();
        service
            .actions
            .lock()
            .unwrap()
            .push(PlatformAccessibilityActionRequest::new(
                button,
                PlatformAccessibilityAction::Invoke,
                snapshot.document_generation().saturating_sub(1),
                snapshot.geometry_revision(),
            ));
        let before = session.document().generation();
        assert_eq!(
            session
                .perform_next_platform_accessibility_action(&service, &mut InvokeExecutor)
                .unwrap_err(),
            PlatformAccessibilityBridgeError::StaleRequest
        );
        assert_eq!(session.document().generation(), before);
    }

    #[test]
    fn current_platform_action_reuses_engine_action_authority() {
        let mut session =
            RenderSession::new("<button>Save</button>", RenderOptions::default()).unwrap();
        let service = RecordingService::default();
        session.sync_platform_accessibility(&service).unwrap();
        let snapshot = service.snapshot.lock().unwrap().clone().unwrap();
        let button = snapshot
            .nodes()
            .iter()
            .find(|node| node.role() == PlatformAccessibilityRole::Button)
            .unwrap()
            .id();
        service
            .actions
            .lock()
            .unwrap()
            .push(PlatformAccessibilityActionRequest::new(
                button,
                PlatformAccessibilityAction::Invoke,
                snapshot.document_generation(),
                snapshot.geometry_revision(),
            ));
        let result = session
            .perform_next_platform_accessibility_action(&service, &mut InvokeExecutor)
            .unwrap()
            .unwrap();
        assert_eq!(result.action(), AccessibilityAction::Invoke);
    }
}
