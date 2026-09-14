mod actions;
mod invalidation;
pub use actions::*;
pub use invalidation::*;

use rarog_dom::{Document, ElementData, Namespace, NodeId, NodeKind};
use rarog_layout::{Fragment, FragmentTree};
use rarog_types::Rect;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_ACCESSIBILITY_NODES: usize = 4096;
pub const DEFAULT_MAX_ACCESSIBILITY_IDENTITIES: usize = 8192;
pub const DEFAULT_MAX_ACCESSIBILITY_DOM_NODES_SCANNED: usize = 16384;
pub const DEFAULT_MAX_ACCESSIBILITY_FRAGMENTS: usize = 16384;
pub const DEFAULT_MAX_ACCESSIBILITY_NAME_BYTES_PER_NODE: usize = 16 * 1024;
pub const DEFAULT_MAX_TOTAL_ACCESSIBILITY_NAME_BYTES: usize = 1024 * 1024;

static NEXT_ACCESSIBILITY_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessibilityLimits {
    pub max_nodes: usize,
    pub max_identities: usize,
    pub max_dom_nodes_scanned: usize,
    pub max_fragments: usize,
    pub max_name_bytes_per_node: usize,
    pub max_total_name_bytes: usize,
}

impl AccessibilityLimits {
    pub const fn is_valid(self) -> bool {
        self.max_nodes > 0
            && self.max_identities >= self.max_nodes
            && self.max_dom_nodes_scanned >= self.max_nodes
            && self.max_fragments > 0
            && self.max_name_bytes_per_node > 0
            && self.max_total_name_bytes >= self.max_name_bytes_per_node
    }
}

impl Default for AccessibilityLimits {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_ACCESSIBILITY_NODES,
            max_identities: DEFAULT_MAX_ACCESSIBILITY_IDENTITIES,
            max_dom_nodes_scanned: DEFAULT_MAX_ACCESSIBILITY_DOM_NODES_SCANNED,
            max_fragments: DEFAULT_MAX_ACCESSIBILITY_FRAGMENTS,
            max_name_bytes_per_node: DEFAULT_MAX_ACCESSIBILITY_NAME_BYTES_PER_NODE,
            max_total_name_bytes: DEFAULT_MAX_TOTAL_ACCESSIBILITY_NAME_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessibilityNodeId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl AccessibilityNodeId {
    pub const fn scope(self) -> u64 {
        self.scope.get()
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
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
pub struct AccessibilityState {
    disabled: bool,
    checked: Option<bool>,
    expanded: Option<bool>,
    focusable: bool,
}

impl AccessibilityState {
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

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityNode {
    id: AccessibilityNodeId,
    source: NodeId,
    role: AccessibilityRole,
    name: String,
    state: AccessibilityState,
    bounds: Option<Rect>,
    parent: Option<AccessibilityNodeId>,
    children: Vec<AccessibilityNodeId>,
}

impl AccessibilityNode {
    pub const fn id(&self) -> AccessibilityNodeId {
        self.id
    }

    pub const fn source(&self) -> NodeId {
        self.source
    }

    pub const fn role(&self) -> AccessibilityRole {
        self.role
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> AccessibilityState {
        self.state
    }

    pub const fn bounds(&self) -> Option<Rect> {
        self.bounds
    }

    pub const fn parent(&self) -> Option<AccessibilityNodeId> {
        self.parent
    }

    pub fn children(&self) -> &[AccessibilityNodeId] {
        &self.children
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityTree {
    scope: NonZeroU64,
    source_generation: u64,
    root: AccessibilityNodeId,
    nodes: BTreeMap<AccessibilityNodeId, AccessibilityNode>,
    sources: BTreeMap<NodeId, AccessibilityNodeId>,
}

impl AccessibilityTree {
    pub const fn scope(&self) -> u64 {
        self.scope.get()
    }

    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }

    pub const fn root(&self) -> AccessibilityNodeId {
        self.root
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node(&self, id: AccessibilityNodeId) -> Option<&AccessibilityNode> {
        self.nodes.get(&id)
    }

    pub fn id_for_source(&self, source: NodeId) -> Option<AccessibilityNodeId> {
        self.sources.get(&source).copied()
    }

    pub fn node_for_source(&self, source: NodeId) -> Option<&AccessibilityNode> {
        self.id_for_source(source).and_then(|id| self.node(id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityError {
    InvalidLimits,
    ScopeExhausted,
    IdentitySpaceExhausted,
    NodeLimitExceeded {
        nodes: usize,
        limit: usize,
    },
    IdentityLimitExceeded {
        identities: usize,
        limit: usize,
    },
    DomNodeLimitExceeded {
        nodes: usize,
        limit: usize,
    },
    FragmentLimitExceeded {
        fragments: usize,
        limit: usize,
    },
    NameByteLimitExceeded {
        node: NodeId,
        bytes: usize,
        limit: usize,
    },
    TotalNameByteLimitExceeded {
        bytes: usize,
        limit: usize,
    },
    InvalidBounds(NodeId),
    InconsistentTree,
}

impl fmt::Display for AccessibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "accessibility tree error: {self:?}")
    }
}

impl std::error::Error for AccessibilityError {}

#[derive(Clone, Debug)]
struct DraftNode {
    source: NodeId,
    role: AccessibilityRole,
    name: String,
    state: AccessibilityState,
    bounds: Option<Rect>,
}

#[derive(Clone, Debug)]
pub struct AccessibilityTreeState {
    limits: AccessibilityLimits,
    scope: NonZeroU64,
    next_serial: u64,
    identities: BTreeMap<NodeId, AccessibilityNodeId>,
}

impl AccessibilityTreeState {
    pub fn try_new(limits: AccessibilityLimits) -> Result<Self, AccessibilityError> {
        if !limits.is_valid() {
            return Err(AccessibilityError::InvalidLimits);
        }
        Ok(Self {
            limits,
            scope: allocate_scope()?,
            next_serial: 1,
            identities: BTreeMap::new(),
        })
    }

    pub const fn limits(&self) -> AccessibilityLimits {
        self.limits
    }

    pub const fn scope(&self) -> u64 {
        self.scope.get()
    }

    pub fn identity_count(&self) -> usize {
        self.identities.len()
    }

    #[cfg(test)]
    fn retained_id(&self, source: NodeId) -> Option<AccessibilityNodeId> {
        self.identities.get(&source).copied()
    }

    pub fn build(
        &mut self,
        document: &Document,
        fragments: &FragmentTree,
    ) -> Result<AccessibilityTree, AccessibilityError> {
        let bounds = collect_fragment_bounds(fragments, self.limits.max_fragments)?;
        let mut drafts = Vec::new();
        let mut total_name_bytes = 0usize;
        let mut stack = vec![document.root()];
        let mut dom_nodes_queued = 1usize;

        while let Some(source) = stack.pop() {
            let node = document
                .node(source)
                .ok_or(AccessibilityError::InconsistentTree)?;
            push_dom_children(
                &mut stack,
                &node.children,
                &mut dom_nodes_queued,
                self.limits.max_dom_nodes_scanned,
            )?;

            let node_bounds = if source == document.root() {
                match bounds.get(&source).copied() {
                    Some(bounds) => Some(bounds),
                    None => Some(validated_rect(fragments.root.boxes.border_box, source)?),
                }
            } else {
                bounds.get(&source).copied()
            };
            if source != document.root() && node_bounds.is_none() {
                continue;
            }

            let next_nodes =
                drafts
                    .len()
                    .checked_add(1)
                    .ok_or(AccessibilityError::NodeLimitExceeded {
                        nodes: usize::MAX,
                        limit: self.limits.max_nodes,
                    })?;
            if next_nodes > self.limits.max_nodes {
                return Err(AccessibilityError::NodeLimitExceeded {
                    nodes: next_nodes,
                    limit: self.limits.max_nodes,
                });
            }
            let role = role_for_node(&node.kind);
            let name = name_for_node(
                document,
                source,
                &node.kind,
                role,
                &bounds,
                self.limits.max_name_bytes_per_node,
                self.limits.max_dom_nodes_scanned,
            )?;
            if matches!(node.kind, NodeKind::Text(_)) && name.is_empty() {
                continue;
            }
            total_name_bytes = total_name_bytes.checked_add(name.len()).ok_or(
                AccessibilityError::TotalNameByteLimitExceeded {
                    bytes: usize::MAX,
                    limit: self.limits.max_total_name_bytes,
                },
            )?;
            if total_name_bytes > self.limits.max_total_name_bytes {
                return Err(AccessibilityError::TotalNameByteLimitExceeded {
                    bytes: total_name_bytes,
                    limit: self.limits.max_total_name_bytes,
                });
            }
            drafts.push(DraftNode {
                source,
                role,
                name,
                state: state_for_node(&node.kind, role),
                bounds: node_bounds,
            });
        }

        let root_source = document.root();
        if !drafts.iter().any(|draft| draft.source == root_source) {
            return Err(AccessibilityError::InconsistentTree);
        }

        let mut preview_identities = self.identities.clone();
        let new_identity_count = drafts
            .iter()
            .filter(|draft| !preview_identities.contains_key(&draft.source))
            .count();
        let resulting_identity_count = preview_identities
            .len()
            .checked_add(new_identity_count)
            .ok_or(AccessibilityError::IdentityLimitExceeded {
                identities: usize::MAX,
                limit: self.limits.max_identities,
            })?;
        if resulting_identity_count > self.limits.max_identities {
            return Err(AccessibilityError::IdentityLimitExceeded {
                identities: resulting_identity_count,
                limit: self.limits.max_identities,
            });
        }

        let mut next_serial = self.next_serial;
        for draft in &drafts {
            if preview_identities.contains_key(&draft.source) {
                continue;
            }
            let serial =
                NonZeroU64::new(next_serial).ok_or(AccessibilityError::IdentitySpaceExhausted)?;
            next_serial = next_serial
                .checked_add(1)
                .ok_or(AccessibilityError::IdentitySpaceExhausted)?;
            preview_identities.insert(
                draft.source,
                AccessibilityNodeId {
                    scope: self.scope,
                    serial,
                },
            );
        }

        let exposed_sources = drafts
            .iter()
            .map(|draft| draft.source)
            .collect::<BTreeSet<_>>();
        let mut nodes = BTreeMap::new();
        let mut sources = BTreeMap::new();
        for draft in &drafts {
            let id = *preview_identities
                .get(&draft.source)
                .ok_or(AccessibilityError::InconsistentTree)?;
            sources.insert(draft.source, id);
            nodes.insert(
                id,
                AccessibilityNode {
                    id,
                    source: draft.source,
                    role: draft.role,
                    name: draft.name.clone(),
                    state: draft.state,
                    bounds: draft.bounds,
                    parent: None,
                    children: Vec::new(),
                },
            );
        }

        for draft in drafts.iter().filter(|draft| draft.source != root_source) {
            let parent_source = nearest_exposed_parent(document, draft.source, &exposed_sources)
                .ok_or(AccessibilityError::InconsistentTree)?;
            let parent_id = *sources
                .get(&parent_source)
                .ok_or(AccessibilityError::InconsistentTree)?;
            let child_id = *sources
                .get(&draft.source)
                .ok_or(AccessibilityError::InconsistentTree)?;
            nodes
                .get_mut(&child_id)
                .ok_or(AccessibilityError::InconsistentTree)?
                .parent = Some(parent_id);
            nodes
                .get_mut(&parent_id)
                .ok_or(AccessibilityError::InconsistentTree)?
                .children
                .push(child_id);
        }

        let root = *sources
            .get(&root_source)
            .ok_or(AccessibilityError::InconsistentTree)?;
        let tree = AccessibilityTree {
            scope: self.scope,
            source_generation: document.generation(),
            root,
            nodes,
            sources,
        };
        self.identities = preview_identities;
        self.next_serial = next_serial;
        Ok(tree)
    }
}

fn push_dom_children(
    stack: &mut Vec<NodeId>,
    children: &[NodeId],
    queued: &mut usize,
    limit: usize,
) -> Result<(), AccessibilityError> {
    for child in children.iter().rev().copied() {
        let next = queued
            .checked_add(1)
            .ok_or(AccessibilityError::DomNodeLimitExceeded {
                nodes: usize::MAX,
                limit,
            })?;
        if next > limit {
            return Err(AccessibilityError::DomNodeLimitExceeded { nodes: next, limit });
        }
        stack.push(child);
        *queued = next;
    }
    Ok(())
}

fn allocate_scope() -> Result<NonZeroU64, AccessibilityError> {
    let scope = NEXT_ACCESSIBILITY_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| AccessibilityError::ScopeExhausted)?;
    NonZeroU64::new(scope).ok_or(AccessibilityError::ScopeExhausted)
}

fn collect_fragment_bounds(
    tree: &FragmentTree,
    max_fragments: usize,
) -> Result<BTreeMap<NodeId, Rect>, AccessibilityError> {
    let mut bounds = BTreeMap::new();
    let mut stack: Vec<&Fragment> = vec![&tree.root];
    let mut queued = 1usize;
    while let Some(fragment) = stack.pop() {
        for child in fragment.children.iter().rev() {
            let next = queued
                .checked_add(1)
                .ok_or(AccessibilityError::FragmentLimitExceeded {
                    fragments: usize::MAX,
                    limit: max_fragments,
                })?;
            if next > max_fragments {
                return Err(AccessibilityError::FragmentLimitExceeded {
                    fragments: next,
                    limit: max_fragments,
                });
            }
            stack.push(child);
            queued = next;
        }
        let Some(source) = fragment.dom_node else {
            continue;
        };
        let rect = validated_rect(fragment.boxes.border_box, source)?;
        if let Some(current) = bounds.get_mut(&source) {
            *current = union_rect(*current, rect, source)?;
        } else {
            bounds.insert(source, rect);
        }
    }
    Ok(bounds)
}

fn validated_rect(rect: Rect, source: NodeId) -> Result<Rect, AccessibilityError> {
    if !rect.origin.x.is_finite()
        || !rect.origin.y.is_finite()
        || !rect.size.width.is_finite()
        || !rect.size.height.is_finite()
        || rect.size.width < 0.0
        || rect.size.height < 0.0
        || !(rect.origin.x + rect.size.width).is_finite()
        || !(rect.origin.y + rect.size.height).is_finite()
    {
        return Err(AccessibilityError::InvalidBounds(source));
    }
    Ok(rect)
}

fn union_rect(left: Rect, right: Rect, source: NodeId) -> Result<Rect, AccessibilityError> {
    let min_x = left.origin.x.min(right.origin.x);
    let min_y = left.origin.y.min(right.origin.y);
    let max_x = (left.origin.x + left.size.width).max(right.origin.x + right.size.width);
    let max_y = (left.origin.y + left.size.height).max(right.origin.y + right.size.height);
    validated_rect(
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y),
        source,
    )
}

fn nearest_exposed_parent(
    document: &Document,
    source: NodeId,
    exposed: &BTreeSet<NodeId>,
) -> Option<NodeId> {
    let mut parent = document.node(source)?.parent;
    let mut steps = 0usize;
    while let Some(current) = parent {
        steps = steps.checked_add(1)?;
        if steps > document.node_count() {
            return None;
        }
        if exposed.contains(&current) {
            return Some(current);
        }
        parent = document.node(current)?.parent;
    }
    None
}

fn role_for_node(kind: &NodeKind) -> AccessibilityRole {
    match kind {
        NodeKind::Document => AccessibilityRole::RootWebArea,
        NodeKind::Text(_) => AccessibilityRole::StaticText,
        NodeKind::Element(element) => role_for_element(element),
    }
}

fn role_for_element(element: &ElementData) -> AccessibilityRole {
    if element.namespace != Namespace::Html {
        return AccessibilityRole::GenericContainer;
    }
    let tag = element.tag_name.as_str();
    if tag.eq_ignore_ascii_case("button") {
        return AccessibilityRole::Button;
    }
    if tag.eq_ignore_ascii_case("a") && element.attributes.contains_key("href") {
        return AccessibilityRole::Link;
    }
    if matches_ascii_case(tag, &["h1", "h2", "h3", "h4", "h5", "h6"]) {
        return AccessibilityRole::Heading;
    }
    if tag.eq_ignore_ascii_case("textarea") {
        return AccessibilityRole::TextField;
    }
    if tag.eq_ignore_ascii_case("input") {
        let input_type = element
            .attributes
            .get("type")
            .map(String::as_str)
            .unwrap_or("text");
        if input_type.eq_ignore_ascii_case("checkbox") {
            return AccessibilityRole::CheckBox;
        }
        if input_type.eq_ignore_ascii_case("radio") {
            return AccessibilityRole::RadioButton;
        }
        if matches_ascii_case(input_type, &["button", "submit", "reset"]) {
            return AccessibilityRole::Button;
        }
        if matches_ascii_case(
            input_type,
            &["text", "search", "email", "url", "tel", "password"],
        ) {
            return AccessibilityRole::TextField;
        }
        return AccessibilityRole::GenericContainer;
    }
    if tag.eq_ignore_ascii_case("img") {
        return AccessibilityRole::Image;
    }
    if tag.eq_ignore_ascii_case("ul") || tag.eq_ignore_ascii_case("ol") {
        return AccessibilityRole::List;
    }
    if tag.eq_ignore_ascii_case("li") {
        return AccessibilityRole::ListItem;
    }
    AccessibilityRole::GenericContainer
}

fn state_for_node(kind: &NodeKind, role: AccessibilityRole) -> AccessibilityState {
    let NodeKind::Element(element) = kind else {
        return AccessibilityState::default();
    };
    let aria_disabled = boolean_attribute_value(element, "aria-disabled");
    let native_disabled = matches!(
        role,
        AccessibilityRole::Button
            | AccessibilityRole::TextField
            | AccessibilityRole::CheckBox
            | AccessibilityRole::RadioButton
    ) && element.attributes.contains_key("disabled");
    let disabled = native_disabled || aria_disabled == Some(true);
    let expanded = boolean_attribute_value(element, "aria-expanded");
    let checked = boolean_attribute_value(element, "aria-checked").or_else(|| {
        matches!(
            role,
            AccessibilityRole::CheckBox | AccessibilityRole::RadioButton
        )
        .then_some(element.attributes.contains_key("checked"))
    });
    let native_focusable = match role {
        AccessibilityRole::Button
        | AccessibilityRole::TextField
        | AccessibilityRole::CheckBox
        | AccessibilityRole::RadioButton => true,
        AccessibilityRole::Link => element.attributes.contains_key("href"),
        _ => false,
    };
    let tabindex_focusable = element
        .attributes
        .get("tabindex")
        .and_then(|value| value.trim().parse::<i32>().ok())
        .is_some_and(|value| value >= 0);
    AccessibilityState {
        disabled,
        checked,
        expanded,
        focusable: !disabled && (native_focusable || tabindex_focusable),
    }
}

fn boolean_attribute_value(element: &ElementData, name: &str) -> Option<bool> {
    let value = element.attributes.get(name)?.trim();
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn name_for_node(
    document: &Document,
    source: NodeId,
    kind: &NodeKind,
    role: AccessibilityRole,
    rendered_bounds: &BTreeMap<NodeId, Rect>,
    limit: usize,
    dom_node_limit: usize,
) -> Result<String, AccessibilityError> {
    match kind {
        NodeKind::Document => Ok(String::new()),
        NodeKind::Text(text) => normalized_text(source, text, limit),
        NodeKind::Element(element) => {
            if let Some(label) = element.attributes.get("aria-label") {
                let normalized = normalized_text(source, label, limit)?;
                if !normalized.is_empty() {
                    return Ok(normalized);
                }
            }
            match role {
                AccessibilityRole::Image => element
                    .attributes
                    .get("alt")
                    .map(|value| normalized_text(source, value, limit))
                    .transpose()
                    .map(|value| value.unwrap_or_default()),
                AccessibilityRole::Button
                    if element.tag_name.as_str().eq_ignore_ascii_case("input") =>
                {
                    element
                        .attributes
                        .get("value")
                        .map(|value| normalized_text(source, value, limit))
                        .transpose()
                        .map(|value| value.unwrap_or_default())
                }
                AccessibilityRole::Button
                | AccessibilityRole::Link
                | AccessibilityRole::Heading
                | AccessibilityRole::ListItem => {
                    descendant_text_name(document, source, rendered_bounds, limit, dom_node_limit)
                }
                _ => Ok(String::new()),
            }
        }
    }
}

fn descendant_text_name(
    document: &Document,
    source: NodeId,
    rendered_bounds: &BTreeMap<NodeId, Rect>,
    limit: usize,
    dom_node_limit: usize,
) -> Result<String, AccessibilityError> {
    let mut accumulator = NameAccumulator::new(source, limit);
    let mut stack = Vec::new();
    let mut dom_nodes_queued = 1usize;
    let children = document
        .children(source)
        .ok_or(AccessibilityError::InconsistentTree)?;
    push_dom_children(&mut stack, children, &mut dom_nodes_queued, dom_node_limit)?;
    while let Some(current) = stack.pop() {
        let node = document
            .node(current)
            .ok_or(AccessibilityError::InconsistentTree)?;
        match &node.kind {
            NodeKind::Text(text) => {
                if rendered_bounds.contains_key(&current) {
                    accumulator.push_text(text)?;
                }
            }
            NodeKind::Document | NodeKind::Element(_) => push_dom_children(
                &mut stack,
                &node.children,
                &mut dom_nodes_queued,
                dom_node_limit,
            )?,
        }
    }
    Ok(accumulator.finish())
}

fn normalized_text(source: NodeId, text: &str, limit: usize) -> Result<String, AccessibilityError> {
    let mut accumulator = NameAccumulator::new(source, limit);
    accumulator.push_text(text)?;
    Ok(accumulator.finish())
}

struct NameAccumulator {
    source: NodeId,
    limit: usize,
    value: String,
    pending_space: bool,
}

impl NameAccumulator {
    fn new(source: NodeId, limit: usize) -> Self {
        Self {
            source,
            limit,
            value: String::new(),
            pending_space: false,
        }
    }

    fn push_text(&mut self, text: &str) -> Result<(), AccessibilityError> {
        for character in text.chars() {
            if character.is_whitespace() {
                if !self.value.is_empty() {
                    self.pending_space = true;
                }
                continue;
            }
            if self.pending_space && !self.value.is_empty() {
                self.push_character(' ')?;
                self.pending_space = false;
            }
            self.push_character(character)?;
        }
        Ok(())
    }

    fn push_character(&mut self, character: char) -> Result<(), AccessibilityError> {
        let bytes = self.value.len().checked_add(character.len_utf8()).ok_or(
            AccessibilityError::NameByteLimitExceeded {
                node: self.source,
                bytes: usize::MAX,
                limit: self.limit,
            },
        )?;
        if bytes > self.limit {
            return Err(AccessibilityError::NameByteLimitExceeded {
                node: self.source,
                bytes,
                limit: self.limit,
            });
        }
        self.value.push(character);
        Ok(())
    }

    fn finish(self) -> String {
        self.value
    }
}

fn matches_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_dom::{ElementData, NodeKind};
    use rarog_layout::{fragments_for_dom, layout_document};
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

    fn small_limits() -> AccessibilityLimits {
        AccessibilityLimits {
            max_nodes: 32,
            max_identities: 64,
            max_dom_nodes_scanned: 128,
            max_fragments: 128,
            max_name_bytes_per_node: 128,
            max_total_name_bytes: 1024,
        }
    }

    #[test]
    fn stable_identity_survives_rebuild_detach_and_reattach() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        document
            .append_new(button, NodeKind::Text("Save".into()))
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();

        let first_layout = layout_document(&document, viewport());
        let first = state.build(&document, &first_layout.fragments).unwrap();
        let first_id = first.id_for_source(button).unwrap();

        document.set_attribute(button, "disabled", "").unwrap();
        let second_layout = layout_document(&document, viewport());
        let second = state.build(&document, &second_layout.fragments).unwrap();
        assert_eq!(second.id_for_source(button), Some(first_id));
        assert!(second.node(first_id).unwrap().state().disabled());

        document.detach(button).unwrap();
        let detached_layout = layout_document(&document, viewport());
        let detached = state.build(&document, &detached_layout.fragments).unwrap();
        assert!(detached.node(first_id).is_none());
        assert_eq!(state.retained_id(button), Some(first_id));

        document.append_child(body, button).unwrap();
        let reattached_layout = layout_document(&document, viewport());
        let reattached = state
            .build(&document, &reattached_layout.fragments)
            .unwrap();
        assert_eq!(reattached.id_for_source(button), Some(first_id));
    }

    #[test]
    fn independent_document_states_never_alias_ids() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut first = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let mut second = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let first_tree = first.build(&document, &layout.fragments).unwrap();
        let second_tree = second.build(&document, &layout.fragments).unwrap();
        assert_ne!(first.scope(), second.scope());
        assert_ne!(
            first_tree.id_for_source(body),
            second_tree.id_for_source(body)
        );
        assert!(
            first_tree
                .node(second_tree.id_for_source(body).unwrap())
                .is_none()
        );
    }

    #[test]
    fn roles_names_states_and_bounds_are_derived_from_rarog_state() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        document
            .set_attribute(button, "aria-label", "  Save   changes  ")
            .unwrap();
        document.set_attribute(button, "disabled", "").unwrap();
        document
            .set_attribute(button, "aria-expanded", "true")
            .unwrap();
        let checkbox = document.append_new(body, element("input")).unwrap();
        document
            .set_attribute(checkbox, "type", "checkbox")
            .unwrap();
        document.set_attribute(checkbox, "checked", "").unwrap();

        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        let button_node = tree.node_for_source(button).unwrap();
        assert_eq!(button_node.role(), AccessibilityRole::Button);
        assert_eq!(button_node.name(), "Save changes");
        assert!(button_node.state().disabled());
        assert_eq!(button_node.state().expanded(), Some(true));
        assert!(!button_node.state().focusable());
        assert!(button_node.bounds().is_some());

        let checkbox_node = tree.node_for_source(checkbox).unwrap();
        assert_eq!(checkbox_node.role(), AccessibilityRole::CheckBox);
        assert_eq!(checkbox_node.state().checked(), Some(true));
        assert!(checkbox_node.state().focusable());
    }

    #[test]
    fn fragmented_text_bounds_union_all_fragment_boxes() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let text = document
            .append_new(
                body,
                NodeKind::Text("one two three four five six seven eight nine ten".into()),
            )
            .unwrap();
        let narrow = Size {
            width: 40.0,
            height: 240.0,
        };
        let layout = layout_document(&document, narrow);
        let text_fragments = fragments_for_dom(&layout.fragments, text);
        assert!(text_fragments.len() > 1);
        let expected = text_fragments
            .iter()
            .skip(1)
            .try_fold(text_fragments[0].boxes.border_box, |current, fragment| {
                union_rect(current, fragment.boxes.border_box, text)
            })
            .unwrap();

        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        assert_eq!(tree.node_for_source(text).unwrap().bounds(), Some(expected));
    }

    #[test]
    fn disconnected_sources_are_absent_and_stale_ids_do_not_resolve() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let link = document.append_new(body, element("a")).unwrap();
        document.set_attribute(link, "href", "/next").unwrap();
        document
            .append_new(link, NodeKind::Text("Next".into()))
            .unwrap();
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let layout = layout_document(&document, viewport());
        let tree = state.build(&document, &layout.fragments).unwrap();
        let stale = tree.id_for_source(link).unwrap();

        document.detach(link).unwrap();
        let layout = layout_document(&document, viewport());
        let rebuilt = state.build(&document, &layout.fragments).unwrap();
        assert!(rebuilt.id_for_source(link).is_none());
        assert!(rebuilt.node(stale).is_none());
    }

    #[test]
    fn nearest_exposed_parent_flattens_unexposed_intermediates() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let wrapper = document.append_new(body, element("div")).unwrap();
        let child = document.append_new(wrapper, element("button")).unwrap();
        let exposed = BTreeSet::from([document.root(), body, child]);
        assert_eq!(
            nearest_exposed_parent(&document, child, &exposed),
            Some(body)
        );
    }

    #[test]
    fn node_budget_failure_does_not_allocate_identities() {
        let mut document = Document::new();
        document
            .append_new(document.root(), element("body"))
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits {
            max_nodes: 1,
            max_identities: 2,
            max_dom_nodes_scanned: 16,
            max_fragments: 16,
            max_name_bytes_per_node: 32,
            max_total_name_bytes: 64,
        })
        .unwrap();
        assert!(matches!(
            state.build(&document, &layout.fragments),
            Err(AccessibilityError::NodeLimitExceeded { .. })
        ));
        assert_eq!(state.identity_count(), 0);
    }

    #[test]
    fn name_budget_failure_does_not_allocate_identities() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        document
            .set_attribute(button, "aria-label", "long label")
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits {
            max_nodes: 8,
            max_identities: 8,
            max_dom_nodes_scanned: 32,
            max_fragments: 32,
            max_name_bytes_per_node: 4,
            max_total_name_bytes: 32,
        })
        .unwrap();
        assert!(matches!(
            state.build(&document, &layout.fragments),
            Err(AccessibilityError::NameByteLimitExceeded { node, .. }) if node == button
        ));
        assert_eq!(state.identity_count(), 0);
    }

    #[test]
    fn identity_budget_failure_is_atomic() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let first = document.append_new(body, element("button")).unwrap();
        let initial_layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits {
            max_nodes: 3,
            max_identities: 3,
            max_dom_nodes_scanned: 32,
            max_fragments: 32,
            max_name_bytes_per_node: 32,
            max_total_name_bytes: 64,
        })
        .unwrap();
        let initial = state.build(&document, &initial_layout.fragments).unwrap();
        let first_id = initial.id_for_source(first).unwrap();
        assert_eq!(state.identity_count(), 3);

        document.append_new(body, element("button")).unwrap();
        let expanded_layout = layout_document(&document, viewport());
        assert!(matches!(
            state.build(&document, &expanded_layout.fragments),
            Err(AccessibilityError::NodeLimitExceeded { .. })
                | Err(AccessibilityError::IdentityLimitExceeded { .. })
        ));
        assert_eq!(state.identity_count(), 3);
        assert_eq!(state.retained_id(first), Some(first_id));
    }

    #[test]
    fn nearest_exposed_parent_is_reflected_in_snapshot() {
        fn clear_source(fragment: &mut Fragment, source: NodeId) {
            if fragment.dom_node == Some(source) {
                fragment.dom_node = None;
            }
            for child in &mut fragment.children {
                clear_source(child, source);
            }
        }

        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let wrapper = document.append_new(body, element("div")).unwrap();
        let child = document.append_new(wrapper, element("button")).unwrap();
        let mut layout = layout_document(&document, viewport());
        clear_source(&mut layout.fragments.root, wrapper);

        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        assert!(tree.node_for_source(wrapper).is_none());
        let body_id = tree.id_for_source(body).unwrap();
        let child_node = tree.node_for_source(child).unwrap();
        assert_eq!(child_node.parent(), Some(body_id));
        assert!(
            tree.node(body_id)
                .unwrap()
                .children()
                .contains(&child_node.id())
        );
    }

    #[test]
    fn invalid_tabindex_does_not_create_focusability() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let generic = document.append_new(body, element("div")).unwrap();
        document
            .set_attribute(generic, "tabindex", "not-a-number")
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        assert!(!tree.node_for_source(generic).unwrap().state().focusable());
    }

    #[test]
    fn root_fallback_bounds_fail_closed_when_invalid() {
        let document = Document::new();
        let mut layout = layout_document(&document, viewport());
        layout.fragments.root.dom_node = None;
        layout.fragments.root.boxes.border_box.size.width = -1.0;
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        assert_eq!(
            state.build(&document, &layout.fragments),
            Err(AccessibilityError::InvalidBounds(document.root()))
        );
        assert_eq!(state.identity_count(), 0);
    }

    #[test]
    fn native_role_mapping_requires_html_namespace() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let svg_button = document
            .append_new(
                body,
                NodeKind::Element(ElementData::new(Namespace::Svg, "button")),
            )
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        assert_eq!(
            tree.node_for_source(svg_button).unwrap().role(),
            AccessibilityRole::GenericContainer
        );
    }

    #[test]
    fn unsupported_input_type_does_not_claim_text_field_semantics() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let range = document.append_new(body, element("input")).unwrap();
        document.set_attribute(range, "type", "range").unwrap();
        document.set_attribute(range, "disabled", "").unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        let node = tree.node_for_source(range).unwrap();
        assert_eq!(node.role(), AccessibilityRole::GenericContainer);
        assert!(!node.state().disabled());
    }

    #[test]
    fn dom_traversal_budget_failure_is_atomic() {
        fn clear_source(fragment: &mut Fragment, source: NodeId) {
            if fragment.dom_node == Some(source) {
                fragment.dom_node = None;
            }
            for child in &mut fragment.children {
                clear_source(child, source);
            }
        }

        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let first = document.append_new(body, element("div")).unwrap();
        let second = document.append_new(first, element("div")).unwrap();
        let mut layout = layout_document(&document, viewport());
        clear_source(&mut layout.fragments.root, first);
        clear_source(&mut layout.fragments.root, second);
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits {
            max_nodes: 2,
            max_identities: 2,
            max_dom_nodes_scanned: 3,
            max_fragments: 32,
            max_name_bytes_per_node: 32,
            max_total_name_bytes: 64,
        })
        .unwrap();
        assert!(matches!(
            state.build(&document, &layout.fragments),
            Err(AccessibilityError::DomNodeLimitExceeded { .. })
        ));
        assert_eq!(state.identity_count(), 0);
    }

    #[test]
    fn fragment_budget_failure_is_atomic() {
        let mut document = Document::new();
        document
            .append_new(document.root(), element("body"))
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits {
            max_nodes: 8,
            max_identities: 8,
            max_dom_nodes_scanned: 16,
            max_fragments: 1,
            max_name_bytes_per_node: 32,
            max_total_name_bytes: 64,
        })
        .unwrap();
        assert!(matches!(
            state.build(&document, &layout.fragments),
            Err(AccessibilityError::FragmentLimitExceeded { .. })
        ));
        assert_eq!(state.identity_count(), 0);
    }

    #[test]
    fn aggregate_name_budget_failure_is_atomic() {
        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let first = document.append_new(body, element("button")).unwrap();
        let second = document.append_new(body, element("button")).unwrap();
        document
            .set_attribute(first, "aria-label", "12345")
            .unwrap();
        document
            .set_attribute(second, "aria-label", "67890")
            .unwrap();
        let layout = layout_document(&document, viewport());
        let mut state = AccessibilityTreeState::try_new(AccessibilityLimits {
            max_nodes: 8,
            max_identities: 8,
            max_dom_nodes_scanned: 16,
            max_fragments: 32,
            max_name_bytes_per_node: 8,
            max_total_name_bytes: 9,
        })
        .unwrap();
        assert!(matches!(
            state.build(&document, &layout.fragments),
            Err(AccessibilityError::TotalNameByteLimitExceeded { .. })
        ));
        assert_eq!(state.identity_count(), 0);
    }

    #[test]
    fn descendant_name_ignores_text_without_rendered_source_geometry() {
        fn clear_source(fragment: &mut Fragment, source: NodeId) {
            if fragment.dom_node == Some(source) {
                fragment.dom_node = None;
            }
            for child in &mut fragment.children {
                clear_source(child, source);
            }
        }

        let mut document = Document::new();
        let body = document
            .append_new(document.root(), element("body"))
            .unwrap();
        let button = document.append_new(body, element("button")).unwrap();
        document
            .append_new(button, NodeKind::Text("Visible".into()))
            .unwrap();
        let hidden_text = document
            .append_new(button, NodeKind::Text("Hidden".into()))
            .unwrap();
        let mut layout = layout_document(&document, viewport());
        clear_source(&mut layout.fragments.root, hidden_text);

        let mut state = AccessibilityTreeState::try_new(small_limits()).unwrap();
        let tree = state.build(&document, &layout.fragments).unwrap();
        assert_eq!(tree.node_for_source(button).unwrap().name(), "Visible");
        assert!(tree.node_for_source(hidden_text).is_none());
    }
}
