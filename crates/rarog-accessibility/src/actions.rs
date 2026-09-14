use super::{
    AccessibilityNodeId, AccessibilityRole, AccessibilityState, AccessibilityTree,
    AccessibilityTreeState, role_for_node, state_for_node,
};
use rarog_dom::{Document, NodeId};
use std::collections::VecDeque;
use std::fmt;

pub const DEFAULT_MAX_ACCESSIBILITY_EVENTS: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityAction {
    Focus,
    Invoke,
    Toggle,
    SetExpanded(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityActionCommand {
    Focus,
    Invoke,
    SetChecked(bool),
    SetExpanded(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityExecutorErrorKind {
    Unavailable,
    Rejected,
    InvalidMutation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityEventKind {
    FocusChanged,
    Invoked,
    StateChanged,
    NameChanged,
    ValueChanged,
    BoundsChanged,
    TreeChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessibilityEvent {
    target: AccessibilityNodeId,
    source: NodeId,
    kind: AccessibilityEventKind,
    document_generation: u64,
}

impl AccessibilityEvent {
    pub(crate) const fn new(
        target: AccessibilityNodeId,
        source: NodeId,
        kind: AccessibilityEventKind,
        document_generation: u64,
    ) -> Self {
        Self {
            target,
            source,
            kind,
            document_generation,
        }
    }

    pub const fn target(self) -> AccessibilityNodeId {
        self.target
    }

    pub const fn source(self) -> NodeId {
        self.source
    }

    pub const fn kind(self) -> AccessibilityEventKind {
        self.kind
    }

    pub const fn document_generation(self) -> u64 {
        self.document_generation
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityEventQueueError {
    InvalidLimit,
    CapacityExceeded { events: usize, limit: usize },
}

impl fmt::Display for AccessibilityEventQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "accessibility event queue error: {self:?}")
    }
}

impl std::error::Error for AccessibilityEventQueueError {}

#[derive(Debug)]
pub struct AccessibilityEventQueue {
    max_events: usize,
    events: VecDeque<AccessibilityEvent>,
}

impl AccessibilityEventQueue {
    pub fn try_with_max_events(max_events: usize) -> Result<Self, AccessibilityEventQueueError> {
        if max_events == 0 {
            return Err(AccessibilityEventQueueError::InvalidLimit);
        }
        Ok(Self {
            max_events,
            events: VecDeque::new(),
        })
    }

    pub const fn max_events(&self) -> usize {
        self.max_events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn pop_front(&mut self) -> Option<AccessibilityEvent> {
        self.events.pop_front()
    }

    pub(crate) fn ensure_capacity(
        &self,
        additional: usize,
    ) -> Result<(), AccessibilityEventQueueError> {
        let events = self.events.len().checked_add(additional).ok_or(
            AccessibilityEventQueueError::CapacityExceeded {
                events: usize::MAX,
                limit: self.max_events,
            },
        )?;
        if events > self.max_events {
            return Err(AccessibilityEventQueueError::CapacityExceeded {
                events,
                limit: self.max_events,
            });
        }
        Ok(())
    }

    pub(crate) fn push_reserved(&mut self, event: AccessibilityEvent) {
        debug_assert!(self.events.len() < self.max_events);
        self.events.push_back(event);
    }
}

impl Default for AccessibilityEventQueue {
    fn default() -> Self {
        Self {
            max_events: DEFAULT_MAX_ACCESSIBILITY_EVENTS,
            events: VecDeque::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityActionError {
    TreeScopeMismatch {
        expected: u64,
        actual: u64,
    },
    TargetScopeMismatch {
        expected: u64,
        actual: u64,
    },
    StaleSnapshot {
        snapshot_generation: u64,
        document_generation: u64,
    },
    TargetNotPresent(AccessibilityNodeId),
    SourceNotLive(NodeId),
    InconsistentAuthority,
    UnsupportedAction {
        role: AccessibilityRole,
        action: AccessibilityAction,
    },
    EventBackpressure {
        events: usize,
        limit: usize,
    },
    Executor(AccessibilityExecutorErrorKind),
    ExecutorContractViolation,
    DomMutationFailed,
}

impl fmt::Display for AccessibilityActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "accessibility action error: {self:?}")
    }
}

impl std::error::Error for AccessibilityActionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessibilityActionResult {
    target: AccessibilityNodeId,
    source: NodeId,
    action: AccessibilityAction,
    event: Option<AccessibilityEvent>,
}

impl AccessibilityActionResult {
    pub const fn target(self) -> AccessibilityNodeId {
        self.target
    }

    pub const fn source(self) -> NodeId {
        self.source
    }

    pub const fn action(self) -> AccessibilityAction {
        self.action
    }

    pub const fn event(self) -> Option<AccessibilityEvent> {
        self.event
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetMutation {
    SetChecked(bool),
    SetExpanded(bool),
}

#[derive(Debug)]
pub struct AccessibilityActionTarget {
    source: NodeId,
    mutation: Option<TargetMutation>,
}

impl AccessibilityActionTarget {
    fn new(source: NodeId) -> Self {
        Self {
            source,
            mutation: None,
        }
    }

    pub const fn source(&self) -> NodeId {
        self.source
    }

    pub fn set_checked(&mut self, checked: bool) -> Result<(), AccessibilityExecutorErrorKind> {
        self.stage_mutation(TargetMutation::SetChecked(checked))
    }

    pub fn set_expanded(&mut self, expanded: bool) -> Result<(), AccessibilityExecutorErrorKind> {
        self.stage_mutation(TargetMutation::SetExpanded(expanded))
    }

    fn stage_mutation(
        &mut self,
        mutation: TargetMutation,
    ) -> Result<(), AccessibilityExecutorErrorKind> {
        match self.mutation {
            None => {
                self.mutation = Some(mutation);
                Ok(())
            }
            Some(current) if current == mutation => Ok(()),
            Some(_) => Err(AccessibilityExecutorErrorKind::InvalidMutation),
        }
    }
}

pub trait AccessibilityActionExecutor {
    fn execute(
        &mut self,
        target: &mut AccessibilityActionTarget,
        command: AccessibilityActionCommand,
    ) -> Result<(), AccessibilityExecutorErrorKind>;
}

#[derive(Debug, Default)]
pub struct DomAccessibilityActionExecutor;

impl AccessibilityActionExecutor for DomAccessibilityActionExecutor {
    fn execute(
        &mut self,
        target: &mut AccessibilityActionTarget,
        command: AccessibilityActionCommand,
    ) -> Result<(), AccessibilityExecutorErrorKind> {
        match command {
            AccessibilityActionCommand::Focus | AccessibilityActionCommand::Invoke => {
                Err(AccessibilityExecutorErrorKind::Unavailable)
            }
            AccessibilityActionCommand::SetChecked(checked) => target.set_checked(checked),
            AccessibilityActionCommand::SetExpanded(expanded) => target.set_expanded(expanded),
        }
    }
}

impl AccessibilityTreeState {
    pub fn perform_action<E: AccessibilityActionExecutor>(
        &self,
        tree: &AccessibilityTree,
        document: &mut Document,
        events: &mut AccessibilityEventQueue,
        executor: &mut E,
        target: AccessibilityNodeId,
        action: AccessibilityAction,
    ) -> Result<AccessibilityActionResult, AccessibilityActionError> {
        let expected_scope = self.scope();
        if tree.scope() != expected_scope {
            return Err(AccessibilityActionError::TreeScopeMismatch {
                expected: expected_scope,
                actual: tree.scope(),
            });
        }
        if target.scope() != expected_scope {
            return Err(AccessibilityActionError::TargetScopeMismatch {
                expected: expected_scope,
                actual: target.scope(),
            });
        }
        if tree.source_generation() != document.generation() {
            return Err(AccessibilityActionError::StaleSnapshot {
                snapshot_generation: tree.source_generation(),
                document_generation: document.generation(),
            });
        }

        let node = tree
            .node(target)
            .ok_or(AccessibilityActionError::TargetNotPresent(target))?;
        let source = node.source();
        let role = node.role();
        let state = node.state();
        if tree.id_for_source(source) != Some(target) {
            return Err(AccessibilityActionError::InconsistentAuthority);
        }
        if !document.is_connected(source) {
            return Err(AccessibilityActionError::SourceNotLive(source));
        }
        let live_node = document
            .node(source)
            .ok_or(AccessibilityActionError::SourceNotLive(source))?;
        let live_role = role_for_node(&live_node.kind);
        let live_state = state_for_node(&live_node.kind, live_role);
        if live_role != role || live_state != state {
            return Err(AccessibilityActionError::InconsistentAuthority);
        }

        let (command, event_kind) = action_plan(role, state, action)?;
        let required_events = usize::from(event_kind.is_some());
        events
            .ensure_capacity(required_events)
            .map_err(|error| match error {
                AccessibilityEventQueueError::InvalidLimit => {
                    AccessibilityActionError::EventBackpressure {
                        events: usize::MAX,
                        limit: events.max_events(),
                    }
                }
                AccessibilityEventQueueError::CapacityExceeded { events, limit } => {
                    AccessibilityActionError::EventBackpressure { events, limit }
                }
            })?;

        if command_requires_dom_generation(command, state) && document.generation() == u64::MAX {
            return Err(AccessibilityActionError::DomMutationFailed);
        }

        let mut action_target = AccessibilityActionTarget::new(source);
        executor
            .execute(&mut action_target, command)
            .map_err(AccessibilityActionError::Executor)?;
        validate_executor_effect(command, action_target.mutation)?;
        apply_target_mutation(document, source, action_target.mutation)?;

        let event = event_kind
            .map(|kind| AccessibilityEvent::new(target, source, kind, document.generation()));
        if let Some(event) = event {
            events.push_reserved(event);
        }
        Ok(AccessibilityActionResult {
            target,
            source,
            action,
            event,
        })
    }
}

fn action_plan(
    role: AccessibilityRole,
    state: AccessibilityState,
    action: AccessibilityAction,
) -> Result<(AccessibilityActionCommand, Option<AccessibilityEventKind>), AccessibilityActionError>
{
    if state.disabled() {
        return Err(AccessibilityActionError::UnsupportedAction { role, action });
    }
    match action {
        AccessibilityAction::Focus if state.focusable() => Ok((
            AccessibilityActionCommand::Focus,
            Some(AccessibilityEventKind::FocusChanged),
        )),
        AccessibilityAction::Invoke
            if matches!(role, AccessibilityRole::Button | AccessibilityRole::Link) =>
        {
            Ok((
                AccessibilityActionCommand::Invoke,
                Some(AccessibilityEventKind::Invoked),
            ))
        }
        AccessibilityAction::Toggle if role == AccessibilityRole::CheckBox => {
            let checked = state
                .checked()
                .ok_or(AccessibilityActionError::InconsistentAuthority)?;
            Ok((
                AccessibilityActionCommand::SetChecked(!checked),
                Some(AccessibilityEventKind::StateChanged),
            ))
        }
        AccessibilityAction::SetExpanded(expanded) => {
            let current = state
                .expanded()
                .ok_or(AccessibilityActionError::UnsupportedAction { role, action })?;
            Ok((
                AccessibilityActionCommand::SetExpanded(expanded),
                (current != expanded).then_some(AccessibilityEventKind::StateChanged),
            ))
        }
        _ => Err(AccessibilityActionError::UnsupportedAction { role, action }),
    }
}

fn command_requires_dom_generation(
    command: AccessibilityActionCommand,
    state: AccessibilityState,
) -> bool {
    match command {
        AccessibilityActionCommand::SetChecked(checked) => state.checked() != Some(checked),
        AccessibilityActionCommand::SetExpanded(expanded) => state.expanded() != Some(expanded),
        AccessibilityActionCommand::Focus | AccessibilityActionCommand::Invoke => false,
    }
}

fn validate_executor_effect(
    command: AccessibilityActionCommand,
    mutation: Option<TargetMutation>,
) -> Result<(), AccessibilityActionError> {
    let valid = match command {
        AccessibilityActionCommand::Focus | AccessibilityActionCommand::Invoke => {
            mutation.is_none()
        }
        AccessibilityActionCommand::SetChecked(checked) => {
            mutation == Some(TargetMutation::SetChecked(checked))
        }
        AccessibilityActionCommand::SetExpanded(expanded) => {
            mutation == Some(TargetMutation::SetExpanded(expanded))
        }
    };
    if valid {
        Ok(())
    } else {
        Err(AccessibilityActionError::ExecutorContractViolation)
    }
}

fn apply_target_mutation(
    document: &mut Document,
    source: NodeId,
    mutation: Option<TargetMutation>,
) -> Result<(), AccessibilityActionError> {
    match mutation {
        None => Ok(()),
        Some(TargetMutation::SetChecked(true)) => document
            .set_attribute(source, "checked", "")
            .map_err(|_| AccessibilityActionError::DomMutationFailed),
        Some(TargetMutation::SetChecked(false)) => document
            .remove_attribute(source, "checked")
            .map(|_| ())
            .map_err(|_| AccessibilityActionError::DomMutationFailed),
        Some(TargetMutation::SetExpanded(expanded)) => document
            .set_attribute(
                source,
                "aria-expanded",
                if expanded { "true" } else { "false" },
            )
            .map_err(|_| AccessibilityActionError::DomMutationFailed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessibilityLimits, AccessibilityTree};
    use rarog_dom::{ElementData, NodeKind};
    use rarog_layout::layout_document;
    use rarog_types::Size;

    fn element(tag: &str) -> NodeKind {
        NodeKind::Element(ElementData::html(tag))
    }

    fn viewport() -> Size {
        Size {
            width: 320.0,
            height: 240.0,
        }
    }

    fn build_tree(state: &mut AccessibilityTreeState, document: &Document) -> AccessibilityTree {
        let layout = layout_document(document, viewport());
        state.build(document, &layout.fragments).unwrap()
    }

    #[derive(Default)]
    struct RecordingExecutor {
        calls: Vec<(NodeId, AccessibilityActionCommand)>,
    }

    impl AccessibilityActionExecutor for RecordingExecutor {
        fn execute(
            &mut self,
            target: &mut AccessibilityActionTarget,
            command: AccessibilityActionCommand,
        ) -> Result<(), AccessibilityExecutorErrorKind> {
            self.calls.push((target.source(), command));
            match command {
                AccessibilityActionCommand::SetChecked(checked) => target.set_checked(checked),
                AccessibilityActionCommand::SetExpanded(expanded) => target.set_expanded(expanded),
                AccessibilityActionCommand::Focus | AccessibilityActionCommand::Invoke => Ok(()),
            }
        }
    }

    struct RejectAfterStage;

    impl AccessibilityActionExecutor for RejectAfterStage {
        fn execute(
            &mut self,
            target: &mut AccessibilityActionTarget,
            command: AccessibilityActionCommand,
        ) -> Result<(), AccessibilityExecutorErrorKind> {
            if let AccessibilityActionCommand::SetChecked(checked) = command {
                target.set_checked(checked)?;
            }
            Err(AccessibilityExecutorErrorKind::Rejected)
        }
    }

    struct IncompleteExecutor;

    impl AccessibilityActionExecutor for IncompleteExecutor {
        fn execute(
            &mut self,
            _target: &mut AccessibilityActionTarget,
            _command: AccessibilityActionCommand,
        ) -> Result<(), AccessibilityExecutorErrorKind> {
            Ok(())
        }
    }

    #[test]
    fn event_queue_rejects_zero_limit() {
        assert_eq!(
            AccessibilityEventQueue::try_with_max_events(0).unwrap_err(),
            AccessibilityEventQueueError::InvalidLimit
        );
    }

    #[test]
    fn foreign_scope_and_foreign_tree_fail_before_executor() {
        let mut document = Document::new();
        let button = document
            .append_new(document.root(), element("button"))
            .unwrap();
        let mut first_state =
            AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let mut second_state =
            AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let first_tree = build_tree(&mut first_state, &document);
        let second_tree = build_tree(&mut second_state, &document);
        let foreign_target = second_tree.id_for_source(button).unwrap();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = RecordingExecutor::default();

        assert!(matches!(
            first_state.perform_action(
                &first_tree,
                &mut document,
                &mut queue,
                &mut executor,
                foreign_target,
                AccessibilityAction::Invoke,
            ),
            Err(AccessibilityActionError::TargetScopeMismatch { .. })
        ));
        assert!(matches!(
            first_state.perform_action(
                &second_tree,
                &mut document,
                &mut queue,
                &mut executor,
                foreign_target,
                AccessibilityAction::Invoke,
            ),
            Err(AccessibilityActionError::TreeScopeMismatch { .. })
        ));
        assert!(executor.calls.is_empty());
        assert!(queue.is_empty());
    }

    #[test]
    fn stale_snapshot_and_absent_identity_fail_closed() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let first = build_tree(&mut state, &document);
        let target = first.id_for_source(button).unwrap();
        document.set_attribute(button, "title", "changed").unwrap();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = RecordingExecutor::default();
        assert!(matches!(
            state.perform_action(
                &first,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            ),
            Err(AccessibilityActionError::StaleSnapshot { .. })
        ));

        let second = build_tree(&mut state, &document);
        document.detach(button).unwrap();
        let third = build_tree(&mut state, &document);
        assert!(third.node(target).is_none());
        assert!(matches!(
            state.perform_action(
                &third,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            ),
            Err(AccessibilityActionError::TargetNotPresent(id)) if id == target
        ));
        assert!(second.node(target).is_some());
        assert!(executor.calls.is_empty());
        assert!(queue.is_empty());
    }

    #[test]
    fn unsupported_action_does_not_reach_executor() {
        let mut document = Document::new();
        let image = document
            .append_new(document.root(), element("img"))
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(image).unwrap();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = RecordingExecutor::default();
        assert!(matches!(
            state.perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            ),
            Err(AccessibilityActionError::UnsupportedAction { .. })
        ));
        assert!(executor.calls.is_empty());
        assert!(queue.is_empty());
    }

    #[test]
    fn exact_dom_source_is_exported_to_replaceable_executor() {
        let mut document = Document::new();
        let button = document
            .append_new(document.root(), element("button"))
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(button).unwrap();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = RecordingExecutor::default();
        let result = state
            .perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            )
            .unwrap();
        assert_eq!(
            executor.calls,
            vec![(button, AccessibilityActionCommand::Invoke)]
        );
        assert_eq!(result.target(), target);
        assert_eq!(result.source(), button);
        assert_eq!(result.action(), AccessibilityAction::Invoke);
        assert_eq!(
            result.event().unwrap().kind(),
            AccessibilityEventKind::Invoked
        );
        assert_eq!(queue.pop_front(), result.event());
    }

    #[test]
    fn event_backpressure_prevents_executor_side_effects() {
        let mut document = Document::new();
        let button = document
            .append_new(document.root(), element("button"))
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(button).unwrap();
        let mut queue = AccessibilityEventQueue::try_with_max_events(1).unwrap();
        let mut executor = RecordingExecutor::default();
        state
            .perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            )
            .unwrap();
        assert_eq!(executor.calls.len(), 1);
        assert!(matches!(
            state.perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            ),
            Err(AccessibilityActionError::EventBackpressure {
                events: 2,
                limit: 1
            })
        ));
        assert_eq!(executor.calls.len(), 1);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn successful_actions_enqueue_events_in_fifo_order() {
        let mut document = Document::new();
        let button = document
            .append_new(document.root(), element("button"))
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(button).unwrap();
        let mut queue = AccessibilityEventQueue::try_with_max_events(2).unwrap();
        let mut executor = RecordingExecutor::default();
        state
            .perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Invoke,
            )
            .unwrap();
        state
            .perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Focus,
            )
            .unwrap();
        assert_eq!(
            queue.pop_front().unwrap().kind(),
            AccessibilityEventKind::Invoked
        );
        assert_eq!(
            queue.pop_front().unwrap().kind(),
            AccessibilityEventKind::FocusChanged
        );
    }

    #[test]
    fn default_dom_executor_applies_checked_and_expanded_mutations() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let checkbox = document.append_new(body, element("input")).unwrap();
        document
            .set_attribute(checkbox, "type", "checkbox")
            .unwrap();
        let expander = document.append_new(body, element("button")).unwrap();
        document
            .set_attribute(expander, "aria-expanded", "false")
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let checkbox_tree = build_tree(&mut state, &document);
        let checkbox_target = checkbox_tree.id_for_source(checkbox).unwrap();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = DomAccessibilityActionExecutor;
        state
            .perform_action(
                &checkbox_tree,
                &mut document,
                &mut queue,
                &mut executor,
                checkbox_target,
                AccessibilityAction::Toggle,
            )
            .unwrap();
        let checked = document.node(checkbox).unwrap();
        let rarog_dom::NodeKind::Element(checked) = &checked.kind else {
            panic!("checkbox must remain an element");
        };
        assert!(checked.attributes.contains_key("checked"));
        assert_eq!(
            queue.pop_front().unwrap().kind(),
            AccessibilityEventKind::StateChanged
        );

        let expander_tree = build_tree(&mut state, &document);
        let expander_target = expander_tree.id_for_source(expander).unwrap();
        state
            .perform_action(
                &expander_tree,
                &mut document,
                &mut queue,
                &mut executor,
                expander_target,
                AccessibilityAction::SetExpanded(true),
            )
            .unwrap();
        let expander_node = document.node(expander).unwrap();
        let rarog_dom::NodeKind::Element(expander_node) = &expander_node.kind else {
            panic!("expander must remain an element");
        };
        assert_eq!(
            expander_node
                .attributes
                .get("aria-expanded")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            queue.pop_front().unwrap().kind(),
            AccessibilityEventKind::StateChanged
        );
    }

    #[test]
    fn executor_failure_discards_staged_dom_mutation_and_event() {
        let mut document = Document::new();
        let checkbox = document
            .append_new(document.root(), element("input"))
            .unwrap();
        document
            .set_attribute(checkbox, "type", "checkbox")
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(checkbox).unwrap();
        let generation = document.generation();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = RejectAfterStage;
        assert_eq!(
            state.perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Toggle,
            ),
            Err(AccessibilityActionError::Executor(
                AccessibilityExecutorErrorKind::Rejected
            ))
        );
        assert_eq!(document.generation(), generation);
        let node = document.node(checkbox).unwrap();
        let rarog_dom::NodeKind::Element(node) = &node.kind else {
            panic!("checkbox must remain an element");
        };
        assert!(!node.attributes.contains_key("checked"));
        assert!(queue.is_empty());
    }

    #[test]
    fn executor_contract_violation_fails_before_dom_mutation() {
        let mut document = Document::new();
        let checkbox = document
            .append_new(document.root(), element("input"))
            .unwrap();
        document
            .set_attribute(checkbox, "type", "checkbox")
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(checkbox).unwrap();
        let generation = document.generation();
        let mut queue = AccessibilityEventQueue::default();
        let mut executor = IncompleteExecutor;
        assert_eq!(
            state.perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::Toggle,
            ),
            Err(AccessibilityActionError::ExecutorContractViolation)
        );
        assert_eq!(document.generation(), generation);
        assert!(queue.is_empty());
    }

    #[test]
    fn same_expanded_value_is_a_bounded_no_event_noop() {
        let mut document = Document::new();
        let button = document
            .append_new(document.root(), element("button"))
            .unwrap();
        document
            .set_attribute(button, "aria-expanded", "false")
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits::default()).unwrap();
        let tree = build_tree(&mut state, &document);
        let target = tree.id_for_source(button).unwrap();
        let generation = document.generation();
        let mut queue = AccessibilityEventQueue::try_with_max_events(1).unwrap();
        let mut executor = RecordingExecutor::default();
        let result = state
            .perform_action(
                &tree,
                &mut document,
                &mut queue,
                &mut executor,
                target,
                AccessibilityAction::SetExpanded(false),
            )
            .unwrap();
        assert_eq!(document.generation(), generation);
        assert!(result.event().is_none());
        assert!(queue.is_empty());
    }
}
