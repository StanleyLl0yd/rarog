use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::{NonZeroIsize, NonZeroUsize};

pub const DEFAULT_MAX_WINDOWS_ACCESSIBILITY_NATIVE_ACTIONS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeErrorKind {
    UnsupportedTarget,
    InvalidWindow,
    InvalidSnapshot,
    CapacityExceeded,
    ProviderUnavailable,
    ComFailure,
    UnsupportedPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsAccessibilityNativeError {
    kind: WindowsAccessibilityNativeErrorKind,
}

impl WindowsAccessibilityNativeError {
    pub(crate) const fn new(kind: WindowsAccessibilityNativeErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> WindowsAccessibilityNativeErrorKind {
        self.kind
    }
}

impl fmt::Display for WindowsAccessibilityNativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Windows accessibility native error: {:?}", self.kind)
    }
}

impl std::error::Error for WindowsAccessibilityNativeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeEventKind {
    FocusChanged,
    Invoked,
    StateChanged,
    NameChanged,
    ValueChanged,
    BoundsChanged,
    TreeChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeAction {
    Focus,
    Invoke,
    Toggle,
    SetExpanded(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsAccessibilityNativeActionRequest {
    provider_serial: u64,
    document_generation: u64,
    geometry_revision: u64,
    action: WindowsAccessibilityNativeAction,
}

impl WindowsAccessibilityNativeActionRequest {
    pub const fn new(
        provider_serial: u64,
        document_generation: u64,
        geometry_revision: u64,
        action: WindowsAccessibilityNativeAction,
    ) -> Self {
        Self {
            provider_serial,
            document_generation,
            geometry_revision,
            action,
        }
    }

    pub const fn provider_serial(self) -> u64 {
        self.provider_serial
    }

    pub const fn document_generation(self) -> u64 {
        self.document_generation
    }

    pub const fn geometry_revision(self) -> u64 {
        self.geometry_revision
    }

    pub const fn action(self) -> WindowsAccessibilityNativeAction {
        self.action
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsAccessibilityNativeRole {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowsAccessibilityNativeRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl WindowsAccessibilityNativeRect {
    pub fn try_new(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<Self, WindowsAccessibilityNativeError> {
        let rect = Self {
            x,
            y,
            width,
            height,
        };
        if !rect.is_valid() {
            return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
        }
        Ok(rect)
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }

    pub const fn width(self) -> f64 {
        self.width
    }

    pub const fn height(self) -> f64 {
        self.height
    }

    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.width && y <= self.y + self.height
    }

    fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width >= 0.0
            && self.height >= 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowsAccessibilityNativeNode {
    provider_serial: u64,
    role: WindowsAccessibilityNativeRole,
    name: String,
    disabled: bool,
    checked: Option<bool>,
    expanded: Option<bool>,
    focusable: bool,
    bounds: Option<WindowsAccessibilityNativeRect>,
    parent: Option<u64>,
    children: Vec<u64>,
}

impl WindowsAccessibilityNativeNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider_serial: u64,
        role: WindowsAccessibilityNativeRole,
        name: String,
        disabled: bool,
        checked: Option<bool>,
        expanded: Option<bool>,
        focusable: bool,
        bounds: Option<WindowsAccessibilityNativeRect>,
        parent: Option<u64>,
        children: Vec<u64>,
    ) -> Self {
        Self {
            provider_serial,
            role,
            name,
            disabled,
            checked,
            expanded,
            focusable,
            bounds,
            parent,
            children,
        }
    }

    pub const fn provider_serial(&self) -> u64 {
        self.provider_serial
    }

    pub const fn role(&self) -> WindowsAccessibilityNativeRole {
        self.role
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    pub const fn checked(&self) -> Option<bool> {
        self.checked
    }

    pub const fn expanded(&self) -> Option<bool> {
        self.expanded
    }

    pub const fn focusable(&self) -> bool {
        self.focusable
    }

    pub const fn bounds(&self) -> Option<WindowsAccessibilityNativeRect> {
        self.bounds
    }

    pub const fn parent(&self) -> Option<u64> {
        self.parent
    }

    pub fn children(&self) -> &[u64] {
        &self.children
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowsAccessibilityNativeSnapshot {
    document_generation: u64,
    geometry_revision: u64,
    root_provider_serial: u64,
    nodes: Vec<WindowsAccessibilityNativeNode>,
}

impl WindowsAccessibilityNativeSnapshot {
    pub fn try_new(
        document_generation: u64,
        geometry_revision: u64,
        root_provider_serial: u64,
        nodes: Vec<WindowsAccessibilityNativeNode>,
    ) -> Result<Self, WindowsAccessibilityNativeError> {
        if geometry_revision == 0 || nodes.is_empty() || !valid_provider_serial(root_provider_serial) {
            return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
        }

        let mut indices = BTreeMap::new();
        for (index, node) in nodes.iter().enumerate() {
            if !valid_provider_serial(node.provider_serial())
                || indices.insert(node.provider_serial(), index).is_some()
                || node.bounds().is_some_and(|bounds| !bounds.is_valid())
            {
                return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
            }
        }
        let Some(&root_index) = indices.get(&root_provider_serial) else {
            return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
        };
        if nodes[root_index].parent().is_some() {
            return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
        }

        for node in &nodes {
            let mut children = BTreeSet::new();
            for &child in node.children() {
                if !children.insert(child) {
                    return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                }
                let Some(&child_index) = indices.get(&child) else {
                    return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                };
                if nodes[child_index].parent() != Some(node.provider_serial()) {
                    return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                }
            }

            match node.parent() {
                Some(parent) => {
                    if node.provider_serial() == root_provider_serial {
                        return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                    }
                    let Some(&parent_index) = indices.get(&parent) else {
                        return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                    };
                    if nodes[parent_index]
                        .children()
                        .iter()
                        .filter(|&&child| child == node.provider_serial())
                        .count()
                        != 1
                    {
                        return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                    }
                }
                None if node.provider_serial() != root_provider_serial => {
                    return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
                }
                None => {}
            }
        }

        let mut visited = BTreeSet::new();
        let mut pending = vec![root_provider_serial];
        while let Some(serial) = pending.pop() {
            if !visited.insert(serial) {
                return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
            }
            let Some(&index) = indices.get(&serial) else {
                return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
            };
            pending.extend(nodes[index].children().iter().rev().copied());
        }
        if visited.len() != nodes.len() {
            return Err(native_error(WindowsAccessibilityNativeErrorKind::InvalidSnapshot));
        }

        Ok(Self {
            document_generation,
            geometry_revision,
            root_provider_serial,
            nodes,
        })
    }

    pub const fn document_generation(&self) -> u64 {
        self.document_generation
    }

    pub const fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    pub const fn root_provider_serial(&self) -> u64 {
        self.root_provider_serial
    }

    pub fn nodes(&self) -> &[WindowsAccessibilityNativeNode] {
        &self.nodes
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct WindowsAccessibilityNativeBridge {
    inner: crate::accessibility_uia::WindowsUiaBridge,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub struct WindowsAccessibilityNativeBridge;

impl WindowsAccessibilityNativeBridge {
    pub fn try_for_window(hwnd: NonZeroIsize) -> Result<Self, WindowsAccessibilityNativeError> {
        Self::try_for_window_with_action_limit(
            hwnd,
            NonZeroUsize::new(DEFAULT_MAX_WINDOWS_ACCESSIBILITY_NATIVE_ACTIONS)
                .expect("non-zero native accessibility action limit"),
        )
    }

    pub fn try_for_window_with_action_limit(
        hwnd: NonZeroIsize,
        max_pending_actions: NonZeroUsize,
    ) -> Result<Self, WindowsAccessibilityNativeError> {
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                inner: crate::accessibility_uia::WindowsUiaBridge::try_new(
                    hwnd,
                    max_pending_actions,
                )?,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (hwnd, max_pending_actions);
            Err(native_error(WindowsAccessibilityNativeErrorKind::UnsupportedTarget))
        }
    }

    pub const fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub fn clear_snapshot(&mut self) -> Result<(), WindowsAccessibilityNativeError> {
        #[cfg(target_os = "windows")]
        {
            self.inner.clear_snapshot()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(native_error(WindowsAccessibilityNativeErrorKind::UnsupportedTarget))
        }
    }

    pub fn replace_snapshot(
        &mut self,
        snapshot: &WindowsAccessibilityNativeSnapshot,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        #[cfg(target_os = "windows")]
        {
            self.inner.replace_snapshot(snapshot)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = snapshot;
            Err(native_error(WindowsAccessibilityNativeErrorKind::UnsupportedTarget))
        }
    }

    pub fn publish_event(
        &mut self,
        provider_serial: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        #[cfg(target_os = "windows")]
        {
            self.inner.publish_event_current(provider_serial, kind)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (provider_serial, kind);
            Err(native_error(WindowsAccessibilityNativeErrorKind::UnsupportedTarget))
        }
    }

    pub fn publish_event_for_snapshot(
        &mut self,
        provider_serial: u64,
        document_generation: u64,
        geometry_revision: u64,
        kind: WindowsAccessibilityNativeEventKind,
    ) -> Result<(), WindowsAccessibilityNativeError> {
        #[cfg(target_os = "windows")]
        {
            self.inner.publish_event(
                provider_serial,
                document_generation,
                geometry_revision,
                kind,
            )
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (
                provider_serial,
                document_generation,
                geometry_revision,
                kind,
            );
            Err(native_error(WindowsAccessibilityNativeErrorKind::UnsupportedTarget))
        }
    }

    pub fn next_action_request(
        &mut self,
    ) -> Result<Option<WindowsAccessibilityNativeActionRequest>, WindowsAccessibilityNativeError>
    {
        #[cfg(target_os = "windows")]
        {
            self.inner.next_action_request()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(native_error(WindowsAccessibilityNativeErrorKind::UnsupportedTarget))
        }
    }
}

pub(crate) const fn native_error(
    kind: WindowsAccessibilityNativeErrorKind,
) -> WindowsAccessibilityNativeError {
    WindowsAccessibilityNativeError::new(kind)
}

pub(crate) const fn valid_provider_serial(serial: u64) -> bool {
    serial != 0 && serial <= i32::MAX as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        serial: u64,
        parent: Option<u64>,
        children: Vec<u64>,
    ) -> WindowsAccessibilityNativeNode {
        WindowsAccessibilityNativeNode::new(
            serial,
            WindowsAccessibilityNativeRole::GenericContainer,
            String::new(),
            false,
            None,
            None,
            false,
            None,
            parent,
            children,
        )
    }

    #[test]
    fn native_snapshot_requires_one_reachable_reciprocal_tree() {
        let valid = WindowsAccessibilityNativeSnapshot::try_new(
            4,
            2,
            1,
            vec![node(1, None, vec![2]), node(2, Some(1), Vec::new())],
        )
        .unwrap();
        assert_eq!(valid.root_provider_serial(), 1);
        assert_eq!(valid.nodes().len(), 2);

        let duplicate = WindowsAccessibilityNativeSnapshot::try_new(
            4,
            2,
            1,
            vec![node(1, None, vec![2, 2]), node(2, Some(1), Vec::new())],
        )
        .unwrap_err();
        assert_eq!(
            duplicate.kind(),
            WindowsAccessibilityNativeErrorKind::InvalidSnapshot
        );
    }

    #[test]
    fn native_snapshot_rejects_unrepresentable_provider_serial() {
        let error = WindowsAccessibilityNativeSnapshot::try_new(
            1,
            1,
            1,
            vec![node(1, None, vec![i32::MAX as u64 + 1]), node(i32::MAX as u64 + 1, Some(1), Vec::new())],
        )
        .unwrap_err();
        assert_eq!(error.kind(), WindowsAccessibilityNativeErrorKind::InvalidSnapshot);
    }

    #[test]
    fn construction_fails_closed_without_a_supported_live_window() {
        let fake = NonZeroIsize::new(1).expect("non-zero test handle");
        let result = WindowsAccessibilityNativeBridge::try_for_window(fake);
        assert_eq!(
            result.unwrap_err().kind(),
            if cfg!(target_os = "windows") {
                WindowsAccessibilityNativeErrorKind::InvalidWindow
            } else {
                WindowsAccessibilityNativeErrorKind::UnsupportedTarget
            }
        );
    }
}
