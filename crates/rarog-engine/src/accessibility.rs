use super::{IncrementalMode, IncrementalReport, RenderError, RenderSession};
use rarog_accessibility::{
    AccessibilityEvent, AccessibilityInvalidation, AccessibilityRefreshError, AccessibilityRuntime,
    AccessibilitySnapshot,
};
use rarog_dom::{Document, MutationKind};
use rarog_layout::FragmentTree;
use rarog_types::Size;

pub(super) struct EngineAccessibilityState {
    runtime: AccessibilityRuntime,
    geometry_revision: u64,
    pending: AccessibilityInvalidation,
    last_error: Option<AccessibilityRefreshError>,
    geometry_exhausted: bool,
}

impl EngineAccessibilityState {
    pub(super) fn try_new(
        document: &Document,
        fragments: &FragmentTree,
    ) -> Result<Self, AccessibilityRefreshError> {
        Ok(Self {
            runtime: AccessibilityRuntime::try_default(document, fragments, 1)?,
            geometry_revision: 1,
            pending: AccessibilityInvalidation::default(),
            last_error: None,
            geometry_exhausted: false,
        })
    }

    fn snapshot_if_current(&self, document: &Document) -> Option<&AccessibilitySnapshot> {
        let snapshot = self.runtime.snapshot();
        (!self.geometry_exhausted
            && snapshot.document_generation() == document.generation()
            && snapshot.geometry_revision() == self.geometry_revision)
            .then_some(snapshot)
    }

    fn note_geometry_change(&mut self) {
        match self.geometry_revision.checked_add(1) {
            Some(next) => self.geometry_revision = next,
            None => {
                self.geometry_exhausted = true;
                self.pending = self
                    .pending
                    .union(AccessibilityInvalidation::full_rebuild());
                self.last_error = Some(AccessibilityRefreshError::InvalidGeometryRevision);
            }
        }
    }

    fn refresh(
        &mut self,
        document: &Document,
        fragments: &FragmentTree,
        invalidation: AccessibilityInvalidation,
    ) {
        let mut combined = self.pending.union(invalidation);
        if self.runtime.snapshot().document_generation() != document.generation() {
            combined = combined.union(AccessibilityInvalidation::semantic());
        }
        if self.runtime.snapshot().geometry_revision() != self.geometry_revision {
            combined = combined.union(AccessibilityInvalidation::bounds());
        }
        if combined.is_empty() || self.geometry_exhausted {
            return;
        }

        match self
            .runtime
            .refresh(document, fragments, self.geometry_revision, combined)
        {
            Ok(_) => {
                self.pending = AccessibilityInvalidation::default();
                self.last_error = None;
            }
            Err(error) => {
                self.pending = combined;
                self.last_error = Some(error);
            }
        }
    }

    fn note_update(
        &mut self,
        document: &Document,
        fragments: &FragmentTree,
        report: IncrementalReport,
        mut invalidation: AccessibilityInvalidation,
    ) {
        match report.mode {
            IncrementalMode::SubtreeRelayout
            | IncrementalMode::FlowRelayout
            | IncrementalMode::GeometryRelayout => {
                self.note_geometry_change();
                invalidation = invalidation
                    .union(AccessibilityInvalidation::tree())
                    .union(AccessibilityInvalidation::bounds());
            }
            IncrementalMode::FullRebuild => {
                self.note_geometry_change();
                invalidation = invalidation.union(AccessibilityInvalidation::full_rebuild());
            }
            IncrementalMode::Unchanged | IncrementalMode::PaintOnlyReuse => {}
        }
        self.refresh(document, fragments, invalidation);
    }

    fn note_resize(&mut self, document: &Document, fragments: &FragmentTree) {
        self.note_geometry_change();
        self.refresh(
            document,
            fragments,
            AccessibilityInvalidation::bounds(),
        );
    }
}

impl RenderSession {
    pub fn accessibility_snapshot(&self) -> Option<&AccessibilitySnapshot> {
        self.accessibility.snapshot_if_current(&self.document)
    }

    pub fn accessibility_is_current(&self) -> bool {
        self.accessibility_snapshot().is_some()
    }

    pub fn accessibility_pending_invalidation(&self) -> AccessibilityInvalidation {
        self.accessibility.pending
    }

    pub fn accessibility_last_error(&self) -> Option<AccessibilityRefreshError> {
        self.accessibility.last_error
    }

    pub fn accessibility_event_count(&self) -> usize {
        self.accessibility.runtime.event_count()
    }

    pub fn next_accessibility_event(&mut self) -> Option<AccessibilityEvent> {
        self.accessibility.runtime.pop_event()
    }

    pub fn accessibility_geometry_revision(&self) -> u64 {
        self.accessibility.geometry_revision
    }

    pub fn resize(&mut self, viewport: Size) -> Result<(), RenderError> {
        self.resize_render_state(viewport)?;
        self.accessibility
            .note_resize(&self.document, &self.layout.fragments);
        Ok(())
    }

    pub fn update(&mut self) -> Result<IncrementalReport, RenderError> {
        let invalidation = accessibility_invalidation_since(
            &self.document,
            self.dirty.through_generation(),
        );
        let report = self.update_render_state()?;
        self.accessibility.note_update(
            &self.document,
            &self.layout.fragments,
            report,
            invalidation,
        );
        Ok(report)
    }
}

fn accessibility_invalidation_since(
    document: &Document,
    from_generation: u64,
) -> AccessibilityInvalidation {
    let Ok(records) = document.mutation_records_since(from_generation) else {
        return AccessibilityInvalidation::full_rebuild();
    };

    records.fold(AccessibilityInvalidation::default(), |current, record| {
        current.union(invalidation_for_mutation(&record.kind))
    })
}

fn invalidation_for_mutation(mutation: &MutationKind) -> AccessibilityInvalidation {
    match mutation {
        MutationKind::NodeCreated { .. }
        | MutationKind::ChildAdded { .. }
        | MutationKind::Reparented { .. } => AccessibilityInvalidation::tree()
            .union(AccessibilityInvalidation::semantic())
            .union(AccessibilityInvalidation::bounds()),
        MutationKind::CharacterData { .. } => AccessibilityInvalidation::semantic()
            .union(AccessibilityInvalidation::bounds()),
        MutationKind::Attribute { name, .. } => {
            let mut invalidation = AccessibilityInvalidation::semantic();
            if matches!(
                name.as_str(),
                "id" | "class" | "style" | "hidden" | "width" | "height"
            ) {
                invalidation = invalidation
                    .union(AccessibilityInvalidation::tree())
                    .union(AccessibilityInvalidation::bounds());
            }
            invalidation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RenderOptions;
    use rarog_accessibility::{AccessibilityEventKind, AccessibilityRole};
    use rarog_dom::{NodeId, NodeKind};

    fn find_tag(document: &Document, tag: &str) -> NodeId {
        let mut stack = vec![document.root()];
        let mut visited = 0usize;
        while let Some(source) = stack.pop() {
            visited += 1;
            assert!(visited <= document.node_count());
            let node = document.node(source).unwrap();
            if matches!(&node.kind, NodeKind::Element(element) if element.tag_name.eq_ignore_ascii_case(tag)) {
                return source;
            }
            stack.extend(node.children.iter().rev().copied());
        }
        panic!("missing <{tag}> node");
    }

    #[test]
    fn render_update_refreshes_accessibility_from_committed_state() {
        let mut session = RenderSession::new(
            "<button id='save'>Save</button>",
            RenderOptions::default(),
        )
        .unwrap();
        let button = find_tag(session.document(), "button");
        let before = session
            .accessibility_snapshot()
            .unwrap()
            .tree()
            .id_for_source(button)
            .unwrap();

        session
            .document_mut()
            .set_attribute(button, "aria-label", "Store")
            .unwrap();
        let report = session.update().unwrap();

        assert_eq!(report.through_generation, session.document().generation());
        let snapshot = session.accessibility_snapshot().unwrap();
        assert_eq!(snapshot.tree().id_for_source(button), Some(before));
        assert_eq!(snapshot.tree().node(before).unwrap().name(), "Store");
        assert_eq!(
            session.next_accessibility_event().unwrap().kind(),
            AccessibilityEventKind::NameChanged
        );
    }

    #[test]
    fn detach_and_reattach_restore_accessibility_identity() {
        let mut session = RenderSession::new(
            "<div><button>Save</button></div>",
            RenderOptions::default(),
        )
        .unwrap();
        let button = find_tag(session.document(), "button");
        let parent = session.document().node(button).unwrap().parent.unwrap();
        let identity = session
            .accessibility_snapshot()
            .unwrap()
            .tree()
            .id_for_source(button)
            .unwrap();

        session.document_mut().detach(button).unwrap();
        session.update().unwrap();
        assert!(session
            .accessibility_snapshot()
            .unwrap()
            .tree()
            .id_for_source(button)
            .is_none());
        assert_eq!(
            session.next_accessibility_event().unwrap().kind(),
            AccessibilityEventKind::TreeChanged
        );

        session.document_mut().append_child(parent, button).unwrap();
        session.update().unwrap();
        assert_eq!(
            session
                .accessibility_snapshot()
                .unwrap()
                .tree()
                .id_for_source(button),
            Some(identity)
        );
    }

    #[test]
    fn resize_advances_geometry_revision_without_changing_semantic_identity() {
        let mut session = RenderSession::new(
            "<button>Save</button>",
            RenderOptions::default(),
        )
        .unwrap();
        let root = session.accessibility_snapshot().unwrap().tree().root();
        let revision = session.accessibility_geometry_revision();

        session
            .resize(Size {
                width: 640.0,
                height: 480.0,
            })
            .unwrap();

        assert_eq!(session.accessibility_geometry_revision(), revision + 1);
        assert_eq!(session.accessibility_snapshot().unwrap().tree().root(), root);
    }

    #[test]
    fn native_role_change_is_derived_after_render_update() {
        let mut session = RenderSession::new(
            "<input type='text'>",
            RenderOptions::default(),
        )
        .unwrap();
        let input = find_tag(session.document(), "input");
        assert_eq!(
            session
                .accessibility_snapshot()
                .unwrap()
                .tree()
                .node_for_source(input)
                .unwrap()
                .role(),
            AccessibilityRole::TextField
        );

        session
            .document_mut()
            .set_attribute(input, "type", "checkbox")
            .unwrap();
        session.update().unwrap();

        assert_eq!(
            session
                .accessibility_snapshot()
                .unwrap()
                .tree()
                .node_for_source(input)
                .unwrap()
                .role(),
            AccessibilityRole::CheckBox
        );
        assert_eq!(
            session.next_accessibility_event().unwrap().kind(),
            AccessibilityEventKind::TreeChanged
        );
    }
}
