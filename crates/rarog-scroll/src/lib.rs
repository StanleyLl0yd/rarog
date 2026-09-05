use rarog_types::{Point, Rect, Size};
use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_SCROLL_NODES: usize = 16_384;

static NEXT_SCROLL_NODE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollTreeLimits {
    max_nodes: NonZeroUsize,
}

impl ScrollTreeLimits {
    pub fn try_new(max_nodes: usize) -> Result<Self, ScrollTreeError> {
        let max_nodes = NonZeroUsize::new(max_nodes).ok_or(ScrollTreeError::InvalidLimits)?;
        Ok(Self { max_nodes })
    }

    pub const fn max_nodes(self) -> usize {
        self.max_nodes.get()
    }
}

impl Default for ScrollTreeLimits {
    fn default() -> Self {
        Self {
            max_nodes: NonZeroUsize::new(DEFAULT_MAX_SCROLL_NODES)
                .expect("default scroll node limit is non-zero"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScrollNodeId(u64);

impl ScrollNodeId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollNodeSnapshot {
    pub id: ScrollNodeId,
    pub parent: Option<ScrollNodeId>,
    pub viewport: Rect,
    pub content_size: Size,
    pub offset: Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollDelta {
    pub node: ScrollNodeId,
    pub previous: Point,
    pub current: Point,
    pub damage: Option<Rect>,
}

impl ScrollDelta {
    pub fn changed(self) -> bool {
        self.previous != self.current
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollTreeError {
    InvalidLimits,
    InvalidGeometry,
    NodeLimitExceeded { nodes: usize, limit: usize },
    UnknownNode(ScrollNodeId),
    IdentitySpaceExhausted,
}

impl fmt::Display for ScrollTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("scroll tree limits must be non-zero"),
            Self::InvalidGeometry => {
                formatter.write_str("scroll geometry must be finite and non-negative")
            }
            Self::NodeLimitExceeded { nodes, limit } => {
                write!(
                    formatter,
                    "scroll tree would contain {nodes} nodes; limit is {limit}"
                )
            }
            Self::UnknownNode(node) => write!(formatter, "unknown scroll node {}", node.get()),
            Self::IdentitySpaceExhausted => {
                formatter.write_str("scroll node identifier space is exhausted")
            }
        }
    }
}

impl std::error::Error for ScrollTreeError {}

#[derive(Clone, Copy, Debug)]
struct ScrollNode {
    parent: Option<ScrollNodeId>,
    viewport: Rect,
    content_size: Size,
    offset: Point,
}

#[derive(Debug)]
pub struct ScrollTree {
    limits: ScrollTreeLimits,
    root: ScrollNodeId,
    nodes: BTreeMap<ScrollNodeId, ScrollNode>,
}

impl ScrollTree {
    pub fn new(
        viewport: Rect,
        content_size: Size,
        limits: ScrollTreeLimits,
    ) -> Result<Self, ScrollTreeError> {
        validate_geometry(viewport, content_size)?;
        let root = allocate_scroll_node_id()?;
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root,
            ScrollNode {
                parent: None,
                viewport,
                content_size,
                offset: Point::default(),
            },
        );
        Ok(Self {
            limits,
            root,
            nodes,
        })
    }

    pub fn with_defaults(viewport: Rect, content_size: Size) -> Result<Self, ScrollTreeError> {
        Self::new(viewport, content_size, ScrollTreeLimits::default())
    }

    pub const fn root(&self) -> ScrollNodeId {
        self.root
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn contains(&self, node: ScrollNodeId) -> bool {
        self.nodes.contains_key(&node)
    }

    pub fn snapshot(&self, node: ScrollNodeId) -> Option<ScrollNodeSnapshot> {
        self.nodes.get(&node).map(|stored| ScrollNodeSnapshot {
            id: node,
            parent: stored.parent,
            viewport: stored.viewport,
            content_size: stored.content_size,
            offset: stored.offset,
        })
    }

    pub fn add_child(
        &mut self,
        parent: ScrollNodeId,
        viewport: Rect,
        content_size: Size,
    ) -> Result<ScrollNodeId, ScrollTreeError> {
        if !self.nodes.contains_key(&parent) {
            return Err(ScrollTreeError::UnknownNode(parent));
        }
        validate_geometry(viewport, content_size)?;

        let nodes = self.nodes.len().saturating_add(1);
        if nodes > self.limits.max_nodes() {
            return Err(ScrollTreeError::NodeLimitExceeded {
                nodes,
                limit: self.limits.max_nodes(),
            });
        }

        let id = allocate_scroll_node_id()?;
        self.nodes.insert(
            id,
            ScrollNode {
                parent: Some(parent),
                viewport,
                content_size,
                offset: Point::default(),
            },
        );
        Ok(id)
    }

    pub fn set_geometry(
        &mut self,
        node: ScrollNodeId,
        viewport: Rect,
        content_size: Size,
    ) -> Result<ScrollDelta, ScrollTreeError> {
        validate_geometry(viewport, content_size)?;
        let stored = self
            .nodes
            .get_mut(&node)
            .ok_or(ScrollTreeError::UnknownNode(node))?;
        let previous = stored.offset;
        stored.viewport = viewport;
        stored.content_size = content_size;
        stored.offset = clamp_offset(viewport, content_size, previous);
        Ok(ScrollDelta {
            node,
            previous,
            current: stored.offset,
            damage: (previous != stored.offset).then_some(viewport),
        })
    }

    pub fn scroll_to(
        &mut self,
        node: ScrollNodeId,
        requested: Point,
    ) -> Result<ScrollDelta, ScrollTreeError> {
        if !point_is_finite(requested) {
            return Err(ScrollTreeError::InvalidGeometry);
        }
        let stored = self
            .nodes
            .get_mut(&node)
            .ok_or(ScrollTreeError::UnknownNode(node))?;
        let previous = stored.offset;
        let current = clamp_offset(stored.viewport, stored.content_size, requested);
        stored.offset = current;
        Ok(ScrollDelta {
            node,
            previous,
            current,
            damage: (previous != current).then_some(stored.viewport),
        })
    }

    pub fn scroll_by(
        &mut self,
        node: ScrollNodeId,
        delta: Point,
    ) -> Result<ScrollDelta, ScrollTreeError> {
        if !point_is_finite(delta) {
            return Err(ScrollTreeError::InvalidGeometry);
        }
        let current = self
            .nodes
            .get(&node)
            .ok_or(ScrollTreeError::UnknownNode(node))?
            .offset;
        let requested = Point {
            x: current.x + delta.x,
            y: current.y + delta.y,
        };
        if !point_is_finite(requested) {
            return Err(ScrollTreeError::InvalidGeometry);
        }
        self.scroll_to(node, requested)
    }

    pub fn remove_subtree(&mut self, node: ScrollNodeId) -> Result<usize, ScrollTreeError> {
        if node == self.root {
            return Ok(0);
        }
        if !self.nodes.contains_key(&node) {
            return Err(ScrollTreeError::UnknownNode(node));
        }

        let mut remove = vec![node];
        let mut cursor = 0;
        while cursor < remove.len() {
            let parent = remove[cursor];
            for (id, candidate) in &self.nodes {
                if candidate.parent == Some(parent) {
                    remove.push(*id);
                }
            }
            cursor += 1;
        }

        let removed = remove.len();
        for id in remove {
            self.nodes.remove(&id);
        }
        Ok(removed)
    }
}

fn allocate_scroll_node_id() -> Result<ScrollNodeId, ScrollTreeError> {
    let raw = NEXT_SCROLL_NODE_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| ScrollTreeError::IdentitySpaceExhausted)?;
    if raw == 0 {
        return Err(ScrollTreeError::IdentitySpaceExhausted);
    }
    Ok(ScrollNodeId(raw))
}

fn validate_geometry(viewport: Rect, content_size: Size) -> Result<(), ScrollTreeError> {
    let values = [
        viewport.origin.x,
        viewport.origin.y,
        viewport.size.width,
        viewport.size.height,
        content_size.width,
        content_size.height,
    ];
    if !values.into_iter().all(f32::is_finite)
        || viewport.size.width < 0.0
        || viewport.size.height < 0.0
        || content_size.width < 0.0
        || content_size.height < 0.0
    {
        return Err(ScrollTreeError::InvalidGeometry);
    }
    Ok(())
}

fn point_is_finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn clamp_offset(viewport: Rect, content_size: Size, requested: Point) -> Point {
    let max_x = (content_size.width - viewport.size.width).max(0.0);
    let max_y = (content_size.height - viewport.size.height).max(0.0);
    Point {
        x: requested.x.clamp(0.0, max_x),
        y: requested.y.clamp(0.0, max_y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rect {
        Rect::new(0.0, 0.0, 100.0, 80.0)
    }

    fn content() -> Size {
        Size {
            width: 300.0,
            height: 240.0,
        }
    }

    #[test]
    fn tree_allocates_stable_non_aliasing_node_ids() {
        let first = ScrollTree::with_defaults(viewport(), content()).unwrap();
        let second = ScrollTree::with_defaults(viewport(), content()).unwrap();
        assert_ne!(first.root(), second.root());
    }

    #[test]
    fn scrolling_clamps_and_reports_viewport_damage() {
        let mut tree = ScrollTree::with_defaults(viewport(), content()).unwrap();
        let root = tree.root();
        let delta = tree.scroll_to(root, Point { x: 500.0, y: -10.0 }).unwrap();
        assert_eq!(delta.previous, Point::default());
        assert_eq!(delta.current, Point { x: 200.0, y: 0.0 });
        assert_eq!(delta.damage, Some(viewport()));

        let unchanged = tree.scroll_to(root, Point { x: 200.0, y: 0.0 }).unwrap();
        assert!(!unchanged.changed());
        assert_eq!(unchanged.damage, None);
    }

    #[test]
    fn geometry_changes_reclamp_existing_offset() {
        let mut tree = ScrollTree::with_defaults(viewport(), content()).unwrap();
        let root = tree.root();
        tree.scroll_to(root, Point { x: 180.0, y: 140.0 }).unwrap();

        let smaller_content = Size {
            width: 120.0,
            height: 100.0,
        };
        let delta = tree
            .set_geometry(root, viewport(), smaller_content)
            .unwrap();
        assert_eq!(delta.current, Point { x: 20.0, y: 20.0 });
        assert_eq!(delta.damage, Some(viewport()));
    }

    #[test]
    fn subtree_removal_preserves_root_and_unrelated_nodes() {
        let mut tree = ScrollTree::with_defaults(viewport(), content()).unwrap();
        let root = tree.root();
        let parent = tree.add_child(root, viewport(), content()).unwrap();
        let child = tree.add_child(parent, viewport(), content()).unwrap();
        let unrelated = tree.add_child(root, viewport(), content()).unwrap();

        assert_eq!(tree.remove_subtree(parent).unwrap(), 2);
        assert!(!tree.contains(parent));
        assert!(!tree.contains(child));
        assert!(tree.contains(unrelated));
        assert!(tree.contains(root));
        assert_eq!(tree.remove_subtree(root).unwrap(), 0);
    }

    #[test]
    fn limits_and_invalid_geometry_fail_before_mutation() {
        let limits = ScrollTreeLimits::try_new(1).unwrap();
        let mut tree = ScrollTree::new(viewport(), content(), limits).unwrap();
        let root = tree.root();
        assert_eq!(
            tree.add_child(root, viewport(), content()).unwrap_err(),
            ScrollTreeError::NodeLimitExceeded { nodes: 2, limit: 1 }
        );
        assert_eq!(tree.len(), 1);

        assert_eq!(
            tree.scroll_to(
                root,
                Point {
                    x: f32::NAN,
                    y: 0.0,
                },
            )
            .unwrap_err(),
            ScrollTreeError::InvalidGeometry
        );
        assert_eq!(tree.snapshot(root).unwrap().offset, Point::default());
    }
}
