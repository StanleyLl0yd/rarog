use super::{
    AccessibilityAction, AccessibilityActionError, AccessibilityActionExecutor,
    AccessibilityActionResult, AccessibilityError, AccessibilityEvent, AccessibilityEventKind,
    AccessibilityEventQueue, AccessibilityEventQueueError, AccessibilityLimits,
    AccessibilityNodeId, AccessibilityTree, AccessibilityTreeState,
};
use rarog_dom::{Document, NodeId};
use rarog_layout::FragmentTree;
use std::fmt;

const TREE_INVALIDATION: u8 = 1 << 0;
const SEMANTIC_INVALIDATION: u8 = 1 << 1;
const BOUNDS_INVALIDATION: u8 = 1 << 2;
const FULL_REBUILD_INVALIDATION: u8 = 1 << 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccessibilityInvalidation {
    bits: u8,
}

impl AccessibilityInvalidation {
    pub const fn tree() -> Self {
        Self {
            bits: TREE_INVALIDATION,
        }
    }

    pub const fn semantic() -> Self {
        Self {
            bits: SEMANTIC_INVALIDATION,
        }
    }

    pub const fn bounds() -> Self {
        Self {
            bits: BOUNDS_INVALIDATION,
        }
    }

    pub const fn full_rebuild() -> Self {
        Self {
            bits: TREE_INVALIDATION
                | SEMANTIC_INVALIDATION
                | BOUNDS_INVALIDATION
                | FULL_REBUILD_INVALIDATION,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub const fn tree_changed(self) -> bool {
        self.bits & TREE_INVALIDATION != 0
    }

    pub const fn semantics_changed(self) -> bool {
        self.bits & SEMANTIC_INVALIDATION != 0
    }

    pub const fn bounds_changed(self) -> bool {
        self.bits & BOUNDS_INVALIDATION != 0
    }

    pub const fn requires_full_rebuild(self) -> bool {
        self.bits & FULL_REBUILD_INVALIDATION != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilitySnapshot {
    tree: AccessibilityTree,
    geometry_revision: u64,
}

impl AccessibilitySnapshot {
    pub fn tree(&self) -> &AccessibilityTree {
        &self.tree
    }

    pub const fn document_generation(&self) -> u64 {
        self.tree.source_generation()
    }

    pub const fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRefreshError {
    InvalidGeometryRevision,
    DocumentGenerationRegressed { current: u64, candidate: u64 },
    GeometryRevisionRegressed { current: u64, candidate: u64 },
    Tree(AccessibilityError),
    EventQueue(AccessibilityEventQueueError),
    ScopeMismatch { expected: u64, actual: u64 },
    IdentityChanged(NodeId),
}

impl fmt::Display for AccessibilityRefreshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "accessibility refresh error: {self:?}")
    }
}

impl std::error::Error for AccessibilityRefreshError {}

impl From<AccessibilityError> for AccessibilityRefreshError {
    fn from(error: AccessibilityError) -> Self {
        Self::Tree(error)
    }
}

impl From<AccessibilityEventQueueError> for AccessibilityRefreshError {
    fn from(error: AccessibilityEventQueueError) -> Self {
        Self::EventQueue(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessibilityRefreshReport {
    invalidation: AccessibilityInvalidation,
    from_document_generation: u64,
    through_document_generation: u64,
    from_geometry_revision: u64,
    through_geometry_revision: u64,
    events_produced: usize,
}

impl AccessibilityRefreshReport {
    pub const fn invalidation(self) -> AccessibilityInvalidation {
        self.invalidation
    }

    pub const fn from_document_generation(self) -> u64 {
        self.from_document_generation
    }

    pub const fn through_document_generation(self) -> u64 {
        self.through_document_generation
    }

    pub const fn from_geometry_revision(self) -> u64 {
        self.from_geometry_revision
    }

    pub const fn through_geometry_revision(self) -> u64 {
        self.through_geometry_revision
    }

    pub const fn events_produced(self) -> usize {
        self.events_produced
    }
}

#[derive(Debug)]
pub struct AccessibilityRuntime {
    tree_state: AccessibilityTreeState,
    snapshot: AccessibilitySnapshot,
    events: AccessibilityEventQueue,
}

impl AccessibilityRuntime {
    pub fn try_new(
        document: &Document,
        fragments: &FragmentTree,
        geometry_revision: u64,
        limits: AccessibilityLimits,
        max_events: usize,
    ) -> Result<Self, AccessibilityRefreshError> {
        if geometry_revision == 0 {
            return Err(AccessibilityRefreshError::InvalidGeometryRevision);
        }
        let mut tree_state = AccessibilityTreeState::try_new(limits)?;
        let tree = tree_state.build(document, fragments)?;
        let events = AccessibilityEventQueue::try_with_max_events(max_events)?;
        Ok(Self {
            tree_state,
            snapshot: AccessibilitySnapshot {
                tree,
                geometry_revision,
            },
            events,
        })
    }

    pub fn try_default(
        document: &Document,
        fragments: &FragmentTree,
        geometry_revision: u64,
    ) -> Result<Self, AccessibilityRefreshError> {
        Self::try_new(
            document,
            fragments,
            geometry_revision,
            AccessibilityLimits::default(),
            super::DEFAULT_MAX_ACCESSIBILITY_EVENTS,
        )
    }

    pub fn snapshot(&self) -> &AccessibilitySnapshot {
        &self.snapshot
    }

    pub const fn scope(&self) -> u64 {
        self.tree_state.scope()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub const fn max_events(&self) -> usize {
        self.events.max_events()
    }

    pub fn pop_event(&mut self) -> Option<AccessibilityEvent> {
        self.events.pop_front()
    }

    pub fn refresh(
        &mut self,
        document: &Document,
        fragments: &FragmentTree,
        geometry_revision: u64,
        invalidation: AccessibilityInvalidation,
    ) -> Result<AccessibilityRefreshReport, AccessibilityRefreshError> {
        if geometry_revision == 0 {
            return Err(AccessibilityRefreshError::InvalidGeometryRevision);
        }
        let expected_scope = self.tree_state.scope();
        if self.snapshot.tree.scope() != expected_scope {
            return Err(AccessibilityRefreshError::ScopeMismatch {
                expected: expected_scope,
                actual: self.snapshot.tree.scope(),
            });
        }

        let from_document_generation = self.snapshot.document_generation();
        let from_geometry_revision = self.snapshot.geometry_revision();
        if document.generation() < from_document_generation {
            return Err(AccessibilityRefreshError::DocumentGenerationRegressed {
                current: from_document_generation,
                candidate: document.generation(),
            });
        }
        if geometry_revision < from_geometry_revision {
            return Err(AccessibilityRefreshError::GeometryRevisionRegressed {
                current: from_geometry_revision,
                candidate: geometry_revision,
            });
        }
        if invalidation.is_empty()
            && from_document_generation == document.generation()
            && from_geometry_revision == geometry_revision
        {
            return Ok(AccessibilityRefreshReport {
                invalidation,
                from_document_generation,
                through_document_generation: from_document_generation,
                from_geometry_revision,
                through_geometry_revision: from_geometry_revision,
                events_produced: 0,
            });
        }

        let mut preview_state = self.tree_state.clone();
        let candidate_tree = preview_state.build(document, fragments)?;
        if candidate_tree.scope() != expected_scope {
            return Err(AccessibilityRefreshError::ScopeMismatch {
                expected: expected_scope,
                actual: candidate_tree.scope(),
            });
        }
        let candidate = AccessibilitySnapshot {
            tree: candidate_tree,
            geometry_revision,
        };
        let staged_events = stage_events(&self.snapshot, &candidate, &self.events)?;

        self.tree_state = preview_state;
        self.snapshot = candidate;
        for event in staged_events.iter().copied() {
            self.events.push_reserved(event);
        }

        Ok(AccessibilityRefreshReport {
            invalidation,
            from_document_generation,
            through_document_generation: self.snapshot.document_generation(),
            from_geometry_revision,
            through_geometry_revision: geometry_revision,
            events_produced: staged_events.len(),
        })
    }

    pub fn perform_action<E: AccessibilityActionExecutor>(
        &mut self,
        document: &mut Document,
        executor: &mut E,
        target: AccessibilityNodeId,
        action: AccessibilityAction,
    ) -> Result<AccessibilityActionResult, AccessibilityActionError> {
        self.tree_state.perform_action(
            &self.snapshot.tree,
            document,
            &mut self.events,
            executor,
            target,
            action,
        )
    }
}

fn stage_events(
    current: &AccessibilitySnapshot,
    candidate: &AccessibilitySnapshot,
    queue: &AccessibilityEventQueue,
) -> Result<Vec<AccessibilityEvent>, AccessibilityRefreshError> {
    let max_detailed_events = candidate
        .tree
        .node_count()
        .checked_mul(3)
        .and_then(|events| events.checked_add(1))
        .ok_or(AccessibilityEventQueueError::CapacityExceeded {
            events: usize::MAX,
            limit: queue.max_events(),
        })?;
    let mut events = Vec::new();
    let mut tree_changed = current.tree.sources.len() != candidate.tree.sources.len();

    for (&source, &current_id) in &current.tree.sources {
        let Some(&candidate_id) = candidate.tree.sources.get(&source) else {
            tree_changed = true;
            continue;
        };
        if current_id != candidate_id {
            return Err(AccessibilityRefreshError::IdentityChanged(source));
        }
        let current_node =
            current
                .tree
                .nodes
                .get(&current_id)
                .ok_or(AccessibilityRefreshError::Tree(
                    AccessibilityError::InconsistentTree,
                ))?;
        let candidate_node =
            candidate
                .tree
                .nodes
                .get(&candidate_id)
                .ok_or(AccessibilityRefreshError::Tree(
                    AccessibilityError::InconsistentTree,
                ))?;

        tree_changed |= current_node.role != candidate_node.role
            || current_node.parent != candidate_node.parent
            || current_node.children != candidate_node.children;
        if current_node.name != candidate_node.name {
            push_staged_event(
                &mut events,
                AccessibilityEvent::new(
                    candidate_id,
                    source,
                    AccessibilityEventKind::NameChanged,
                    candidate.document_generation(),
                ),
                max_detailed_events,
                queue.max_events(),
            )?;
        }
        if current_node.state != candidate_node.state {
            push_staged_event(
                &mut events,
                AccessibilityEvent::new(
                    candidate_id,
                    source,
                    AccessibilityEventKind::StateChanged,
                    candidate.document_generation(),
                ),
                max_detailed_events,
                queue.max_events(),
            )?;
        }
        if current_node.bounds != candidate_node.bounds {
            push_staged_event(
                &mut events,
                AccessibilityEvent::new(
                    candidate_id,
                    source,
                    AccessibilityEventKind::BoundsChanged,
                    candidate.document_generation(),
                ),
                max_detailed_events,
                queue.max_events(),
            )?;
        }
    }

    if !tree_changed {
        tree_changed = candidate
            .tree
            .sources
            .keys()
            .any(|source| !current.tree.sources.contains_key(source));
    }
    if tree_changed {
        events.insert(0, tree_changed_event(candidate)?);
    }

    match queue.ensure_capacity(events.len()) {
        Ok(()) => Ok(events),
        Err(error) if events.is_empty() => Err(error.into()),
        Err(_) => {
            queue.ensure_capacity(1)?;
            Ok(vec![tree_changed_event(candidate)?])
        }
    }
}

fn tree_changed_event(
    candidate: &AccessibilitySnapshot,
) -> Result<AccessibilityEvent, AccessibilityRefreshError> {
    let root = candidate.tree.root();
    let source = candidate
        .tree
        .node(root)
        .ok_or(AccessibilityRefreshError::Tree(
            AccessibilityError::InconsistentTree,
        ))?
        .source();
    Ok(AccessibilityEvent::new(
        root,
        source,
        AccessibilityEventKind::TreeChanged,
        candidate.document_generation(),
    ))
}

fn push_staged_event(
    staged: &mut Vec<AccessibilityEvent>,
    event: AccessibilityEvent,
    max_detailed_events: usize,
    queue_limit: usize,
) -> Result<(), AccessibilityRefreshError> {
    let events =
        staged
            .len()
            .checked_add(1)
            .ok_or(AccessibilityEventQueueError::CapacityExceeded {
                events: usize::MAX,
                limit: queue_limit,
            })?;
    if events > max_detailed_events {
        return Err(AccessibilityEventQueueError::CapacityExceeded {
            events,
            limit: max_detailed_events,
        }
        .into());
    }
    staged.push(event);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccessibilityRole;
    use rarog_dom::{ElementData, NodeKind};
    use rarog_layout::layout_document;
    use rarog_types::Size;

    fn element(tag: &str) -> NodeKind {
        NodeKind::Element(ElementData::html(tag))
    }

    fn viewport(width: f32) -> Size {
        Size {
            width,
            height: 240.0,
        }
    }

    fn runtime(document: &Document) -> AccessibilityRuntime {
        let layout = layout_document(document, viewport(320.0));
        AccessibilityRuntime::try_new(
            document,
            &layout.fragments,
            1,
            AccessibilityLimits {
                max_nodes: 32,
                max_identities: 64,
                max_dom_nodes_scanned: 128,
                max_fragments: 128,
                max_name_bytes_per_node: 128,
                max_total_name_bytes: 1024,
            },
            8,
        )
        .unwrap()
    }

    #[test]
    fn semantic_refresh_preserves_identity_and_emits_name_event() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        let text = document
            .append_new(button, NodeKind::Text("Save".into()))
            .unwrap();
        let mut runtime = runtime(&document);
        let id = runtime.snapshot().tree().id_for_source(button).unwrap();

        document.set_text(text, "Store").unwrap();
        let layout = layout_document(&document, viewport(320.0));
        let report = runtime
            .refresh(
                &document,
                &layout.fragments,
                2,
                AccessibilityInvalidation::semantic().union(AccessibilityInvalidation::bounds()),
            )
            .unwrap();

        assert_eq!(runtime.snapshot().tree().id_for_source(button), Some(id));
        assert!(report.events_produced() >= 1);
        let mut saw_button_name = false;
        while let Some(event) = runtime.pop_event() {
            assert_eq!(event.document_generation(), document.generation());
            saw_button_name |=
                event.target() == id && event.kind() == AccessibilityEventKind::NameChanged;
        }
        assert!(saw_button_name);
    }

    #[test]
    fn membership_refresh_coalesces_to_tree_event_and_restores_identity() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        let mut runtime = runtime(&document);
        let id = runtime.snapshot().tree().id_for_source(button).unwrap();

        document.detach(button).unwrap();
        let detached = layout_document(&document, viewport(320.0));
        runtime
            .refresh(
                &document,
                &detached.fragments,
                2,
                AccessibilityInvalidation::tree().union(AccessibilityInvalidation::bounds()),
            )
            .unwrap();
        assert!(runtime.snapshot().tree().id_for_source(button).is_none());
        assert_eq!(
            runtime.pop_event().unwrap().kind(),
            AccessibilityEventKind::TreeChanged
        );

        document.append_child(body, button).unwrap();
        let attached = layout_document(&document, viewport(320.0));
        runtime
            .refresh(
                &document,
                &attached.fragments,
                3,
                AccessibilityInvalidation::tree().union(AccessibilityInvalidation::bounds()),
            )
            .unwrap();
        assert_eq!(runtime.snapshot().tree().id_for_source(button), Some(id));
        assert_eq!(
            runtime.pop_event().unwrap().kind(),
            AccessibilityEventKind::TreeChanged
        );
    }

    #[test]
    fn bounds_only_refresh_tracks_geometry_revision() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let text = document
            .append_new(body, NodeKind::Text("a long line that can wrap".into()))
            .unwrap();
        let mut runtime = runtime(&document);
        let before = runtime
            .snapshot()
            .tree()
            .node_for_source(text)
            .unwrap()
            .bounds();

        let layout = layout_document(&document, viewport(40.0));
        runtime
            .refresh(
                &document,
                &layout.fragments,
                2,
                AccessibilityInvalidation::bounds(),
            )
            .unwrap();
        let after = runtime
            .snapshot()
            .tree()
            .node_for_source(text)
            .unwrap()
            .bounds();

        assert_eq!(runtime.snapshot().geometry_revision(), 2);
        assert_ne!(before, after);
        let mut saw_bounds = false;
        while let Some(event) = runtime.pop_event() {
            saw_bounds |= event.kind() == AccessibilityEventKind::BoundsChanged;
        }
        assert!(saw_bounds);
    }

    #[test]
    fn backpressure_keeps_snapshot_and_identity_state_atomic() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let first = document.append_new(body, element("button")).unwrap();
        document.set_attribute(first, "aria-label", "One").unwrap();
        let layout = layout_document(&document, viewport(320.0));
        let mut runtime = AccessibilityRuntime::try_new(
            &document,
            &layout.fragments,
            1,
            AccessibilityLimits {
                max_nodes: 32,
                max_identities: 64,
                max_dom_nodes_scanned: 128,
                max_fragments: 128,
                max_name_bytes_per_node: 128,
                max_total_name_bytes: 1024,
            },
            1,
        )
        .unwrap();
        let old_generation = runtime.snapshot().document_generation();
        let old_identity_count = runtime.tree_state.identity_count();
        let root = runtime.snapshot().tree().root();
        let root_source = runtime.snapshot().tree().node(root).unwrap().source();
        runtime.events.push_reserved(AccessibilityEvent::new(
            root,
            root_source,
            AccessibilityEventKind::Invoked,
            old_generation,
        ));

        document.set_attribute(first, "aria-label", "Two").unwrap();
        let second = document.append_new(body, element("button")).unwrap();
        document
            .set_attribute(second, "aria-label", "Three")
            .unwrap();
        let changed = layout_document(&document, viewport(320.0));
        let error = runtime
            .refresh(
                &document,
                &changed.fragments,
                2,
                AccessibilityInvalidation::full_rebuild(),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            AccessibilityRefreshError::EventQueue(
                AccessibilityEventQueueError::CapacityExceeded { .. }
            )
        ));
        assert_eq!(runtime.snapshot().document_generation(), old_generation);
        assert_eq!(runtime.snapshot().geometry_revision(), 1);
        assert_eq!(runtime.tree_state.identity_count(), old_identity_count);
        assert_eq!(runtime.event_count(), 1);
        assert_eq!(
            runtime.pop_event().unwrap().kind(),
            AccessibilityEventKind::Invoked
        );
    }

    #[test]
    fn oversized_detailed_batch_coalesces_to_one_tree_event() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let first = document.append_new(body, element("button")).unwrap();
        let second = document.append_new(body, element("button")).unwrap();
        document.set_attribute(first, "aria-label", "One").unwrap();
        document.set_attribute(second, "aria-label", "Two").unwrap();
        let layout = layout_document(&document, viewport(320.0));
        let mut runtime = AccessibilityRuntime::try_new(
            &document,
            &layout.fragments,
            1,
            AccessibilityLimits {
                max_nodes: 32,
                max_identities: 64,
                max_dom_nodes_scanned: 128,
                max_fragments: 128,
                max_name_bytes_per_node: 128,
                max_total_name_bytes: 1024,
            },
            1,
        )
        .unwrap();

        document
            .set_attribute(first, "aria-label", "First")
            .unwrap();
        document
            .set_attribute(second, "aria-label", "Second")
            .unwrap();
        let changed = layout_document(&document, viewport(320.0));
        let report = runtime
            .refresh(
                &document,
                &changed.fragments,
                2,
                AccessibilityInvalidation::semantic(),
            )
            .unwrap();

        assert_eq!(report.events_produced(), 1);
        assert_eq!(
            runtime
                .snapshot()
                .tree()
                .node_for_source(first)
                .unwrap()
                .name(),
            "First"
        );
        assert_eq!(
            runtime
                .snapshot()
                .tree()
                .node_for_source(second)
                .unwrap()
                .name(),
            "Second"
        );
        assert_eq!(
            runtime.pop_event().unwrap().kind(),
            AccessibilityEventKind::TreeChanged
        );
        assert_eq!(runtime.event_count(), 0);
    }

    #[test]
    fn generation_regression_is_rejected_before_candidate_publication() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        document.append_new(body, element("button")).unwrap();
        let layout = layout_document(&document, viewport(320.0));
        let mut runtime = AccessibilityRuntime::try_new(
            &document,
            &layout.fragments,
            2,
            AccessibilityLimits {
                max_nodes: 32,
                max_identities: 64,
                max_dom_nodes_scanned: 128,
                max_fragments: 128,
                max_name_bytes_per_node: 128,
                max_total_name_bytes: 1024,
            },
            8,
        )
        .unwrap();

        let older = Document::new();
        let older_layout = layout_document(&older, viewport(320.0));
        assert!(matches!(
            runtime.refresh(
                &older,
                &older_layout.fragments,
                2,
                AccessibilityInvalidation::full_rebuild(),
            ),
            Err(AccessibilityRefreshError::DocumentGenerationRegressed { .. })
        ));
        assert!(matches!(
            runtime.refresh(
                &document,
                &layout.fragments,
                1,
                AccessibilityInvalidation::bounds(),
            ),
            Err(AccessibilityRefreshError::GeometryRevisionRegressed { .. })
        ));
        assert_eq!(runtime.snapshot().geometry_revision(), 2);
        assert_eq!(
            runtime.snapshot().document_generation(),
            document.generation()
        );
    }
    #[test]
    fn full_rebuild_matches_direct_builder_for_covered_semantics() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let checkbox = document.append_new(body, element("input")).unwrap();
        document
            .set_attribute(checkbox, "type", "checkbox")
            .unwrap();
        let mut runtime = runtime(&document);

        document.set_attribute(checkbox, "checked", "").unwrap();
        let layout = layout_document(&document, viewport(320.0));
        runtime
            .refresh(
                &document,
                &layout.fragments,
                2,
                AccessibilityInvalidation::full_rebuild(),
            )
            .unwrap();

        let node = runtime.snapshot().tree().node_for_source(checkbox).unwrap();
        assert_eq!(node.role(), AccessibilityRole::CheckBox);
        assert_eq!(node.state().checked(), Some(true));
        assert_eq!(
            runtime.snapshot().document_generation(),
            document.generation()
        );
    }
}
