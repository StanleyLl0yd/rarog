use std::collections::BTreeSet;
use std::fmt;
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlatformAccessibilityNodeId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl PlatformAccessibilityNodeId {
    pub fn try_new(scope: u64, serial: u64) -> Result<Self, PlatformAccessibilityError> {
        Ok(Self {
            scope: NonZeroU64::new(scope).ok_or_else(invalid_snapshot)?,
            serial: NonZeroU64::new(serial).ok_or_else(invalid_snapshot)?,
        })
    }

    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAccessibilityRole {
    RootWebArea,
    GenericContainer,
    StaticText,
    Button,
    Link,
    Heading,
    TextField,
    CheckBox,
    RadioButton,
    Image,
    List,
    ListItem,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformAccessibilityState {
    disabled: bool,
    checked: Option<bool>,
    expanded: Option<bool>,
    focusable: bool,
}

impl PlatformAccessibilityState {
    pub const fn new(
        disabled: bool,
        checked: Option<bool>,
        expanded: Option<bool>,
        focusable: bool,
    ) -> Self {
        Self {
            disabled,
            checked,
            expanded,
            focusable,
        }
    }

    pub const fn disabled(self) -> bool {
        self.disabled
    }

    pub const fn checked(self) -> Option<bool> {
        self.checked
    }

    pub const fn expanded(self) -> Option<bool> {
        self.expanded
    }

    pub const fn focusable(self) -> bool {
        self.focusable
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformAccessibilityRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PlatformAccessibilityRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width >= 0.0
            && self.height >= 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformAccessibilityNode {
    id: PlatformAccessibilityNodeId,
    role: PlatformAccessibilityRole,
    name: String,
    state: PlatformAccessibilityState,
    bounds: Option<PlatformAccessibilityRect>,
    parent: Option<PlatformAccessibilityNodeId>,
    children: Vec<PlatformAccessibilityNodeId>,
}

impl PlatformAccessibilityNode {
    pub fn new(
        id: PlatformAccessibilityNodeId,
        role: PlatformAccessibilityRole,
        name: String,
        state: PlatformAccessibilityState,
        bounds: Option<PlatformAccessibilityRect>,
        parent: Option<PlatformAccessibilityNodeId>,
        children: Vec<PlatformAccessibilityNodeId>,
    ) -> Self {
        Self {
            id,
            role,
            name,
            state,
            bounds,
            parent,
            children,
        }
    }

    pub const fn id(&self) -> PlatformAccessibilityNodeId {
        self.id
    }

    pub const fn role(&self) -> PlatformAccessibilityRole {
        self.role
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> PlatformAccessibilityState {
        self.state
    }

    pub const fn bounds(&self) -> Option<PlatformAccessibilityRect> {
        self.bounds
    }

    pub const fn parent(&self) -> Option<PlatformAccessibilityNodeId> {
        self.parent
    }

    pub fn children(&self) -> &[PlatformAccessibilityNodeId] {
        &self.children
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformAccessibilitySnapshot {
    document_generation: u64,
    geometry_revision: u64,
    root: PlatformAccessibilityNodeId,
    nodes: Vec<PlatformAccessibilityNode>,
}

impl PlatformAccessibilitySnapshot {
    pub fn try_new(
        document_generation: u64,
        geometry_revision: u64,
        root: PlatformAccessibilityNodeId,
        nodes: Vec<PlatformAccessibilityNode>,
    ) -> Result<Self, PlatformAccessibilityError> {
        if geometry_revision == 0 || nodes.is_empty() {
            return Err(invalid_snapshot());
        }
        let scope = root.scope();
        let mut ids = BTreeSet::new();
        for node in &nodes {
            if node.id().scope() != scope || !ids.insert(node.id()) {
                return Err(invalid_snapshot());
            }
            if node.bounds().is_some_and(|bounds| !bounds.is_valid()) {
                return Err(invalid_snapshot());
            }
        }
        if !ids.contains(&root) {
            return Err(invalid_snapshot());
        }
        for node in &nodes {
            if node.parent().is_some_and(|parent| !ids.contains(&parent))
                || node.children().iter().any(|child| !ids.contains(child))
            {
                return Err(invalid_snapshot());
            }
        }
        Ok(Self {
            document_generation,
            geometry_revision,
            root,
            nodes,
        })
    }

    pub const fn document_generation(&self) -> u64 {
        self.document_generation
    }

    pub const fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    pub const fn scope(&self) -> u64 {
        self.root.scope()
    }

    pub const fn root(&self) -> PlatformAccessibilityNodeId {
        self.root
    }

    pub fn nodes(&self) -> &[PlatformAccessibilityNode] {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAccessibilityEventKind {
    FocusChanged,
    Invoked,
    StateChanged,
    NameChanged,
    ValueChanged,
    BoundsChanged,
    TreeChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformAccessibilityEvent {
    target: PlatformAccessibilityNodeId,
    kind: PlatformAccessibilityEventKind,
    document_generation: u64,
}

impl PlatformAccessibilityEvent {
    pub const fn new(
        target: PlatformAccessibilityNodeId,
        kind: PlatformAccessibilityEventKind,
        document_generation: u64,
    ) -> Self {
        Self {
            target,
            kind,
            document_generation,
        }
    }

    pub const fn target(self) -> PlatformAccessibilityNodeId {
        self.target
    }

    pub const fn kind(self) -> PlatformAccessibilityEventKind {
        self.kind
    }

    pub const fn document_generation(self) -> u64 {
        self.document_generation
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAccessibilityAction {
    Focus,
    Invoke,
    Toggle,
    SetExpanded(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformAccessibilityActionRequest {
    target: PlatformAccessibilityNodeId,
    action: PlatformAccessibilityAction,
    document_generation: u64,
    geometry_revision: u64,
}

impl PlatformAccessibilityActionRequest {
    pub const fn new(
        target: PlatformAccessibilityNodeId,
        action: PlatformAccessibilityAction,
        document_generation: u64,
        geometry_revision: u64,
    ) -> Self {
        Self {
            target,
            action,
            document_generation,
            geometry_revision,
        }
    }

    pub const fn target(self) -> PlatformAccessibilityNodeId {
        self.target
    }

    pub const fn action(self) -> PlatformAccessibilityAction {
        self.action
    }

    pub const fn document_generation(self) -> u64 {
        self.document_generation
    }

    pub const fn geometry_revision(self) -> u64 {
        self.geometry_revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAccessibilityErrorKind {
    UnsupportedTarget,
    InvalidSnapshot,
    CapacityExceeded,
    StaleCorrelation,
    NativeUnavailable,
    NativeFailure,
    UnsupportedPattern,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformAccessibilityError {
    kind: PlatformAccessibilityErrorKind,
}

impl PlatformAccessibilityError {
    pub const fn new(kind: PlatformAccessibilityErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> PlatformAccessibilityErrorKind {
        self.kind
    }
}

impl fmt::Display for PlatformAccessibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "platform accessibility error: {:?}", self.kind)
    }
}

impl std::error::Error for PlatformAccessibilityError {}

fn invalid_snapshot() -> PlatformAccessibilityError {
    PlatformAccessibilityError::new(PlatformAccessibilityErrorKind::InvalidSnapshot)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformAccessibilitySnapshotReport {
    pub retained_providers: usize,
    pub created_providers: usize,
    pub retired_providers: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAccessibilityEventDisposition {
    Delivered,
    Queued,
}

pub trait PlatformAccessibilityService: Send + Sync {
    fn clear_snapshot(&self) -> Result<(), PlatformAccessibilityError>;

    fn replace_snapshot(
        &self,
        snapshot: &PlatformAccessibilitySnapshot,
    ) -> Result<PlatformAccessibilitySnapshotReport, PlatformAccessibilityError>;

    fn publish_event(
        &self,
        event: PlatformAccessibilityEvent,
    ) -> Result<PlatformAccessibilityEventDisposition, PlatformAccessibilityError>;

    fn next_action_request(
        &self,
    ) -> Result<Option<PlatformAccessibilityActionRequest>, PlatformAccessibilityError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(serial: u64) -> PlatformAccessibilityNodeId {
        PlatformAccessibilityNodeId::try_new(7, serial).unwrap()
    }

    fn node(serial: u64, parent: Option<u64>) -> PlatformAccessibilityNode {
        PlatformAccessibilityNode::new(
            id(serial),
            PlatformAccessibilityRole::GenericContainer,
            String::new(),
            PlatformAccessibilityState::default(),
            None,
            parent.map(id),
            Vec::new(),
        )
    }

    #[test]
    fn snapshot_rejects_foreign_scope_and_missing_references() {
        let root = id(1);
        let foreign = PlatformAccessibilityNodeId::try_new(8, 2).unwrap();
        let nodes = vec![
            PlatformAccessibilityNode::new(
                root,
                PlatformAccessibilityRole::RootWebArea,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                None,
                vec![foreign],
            ),
            PlatformAccessibilityNode::new(
                foreign,
                PlatformAccessibilityRole::GenericContainer,
                String::new(),
                PlatformAccessibilityState::default(),
                None,
                Some(root),
                Vec::new(),
            ),
        ];
        assert_eq!(
            PlatformAccessibilitySnapshot::try_new(1, 1, root, nodes)
                .unwrap_err()
                .kind(),
            PlatformAccessibilityErrorKind::InvalidSnapshot
        );
    }

    #[test]
    fn snapshot_accepts_bounded_tree_projection_shape() {
        let root = id(1);
        let child = id(2);
        let nodes = vec![
            PlatformAccessibilityNode::new(
                root,
                PlatformAccessibilityRole::RootWebArea,
                "document".into(),
                PlatformAccessibilityState::default(),
                Some(PlatformAccessibilityRect::new(0.0, 0.0, 100.0, 80.0)),
                None,
                vec![child],
            ),
            node(2, Some(1)),
        ];
        let snapshot = PlatformAccessibilitySnapshot::try_new(4, 2, root, nodes).unwrap();
        assert_eq!(snapshot.scope(), 7);
        assert_eq!(snapshot.node_count(), 2);
        assert_eq!(snapshot.document_generation(), 4);
        assert_eq!(snapshot.geometry_revision(), 2);
    }
}
