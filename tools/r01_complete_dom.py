from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:100]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


dom = "crates/rarog-dom/src/lib.rs"
replace(
    dom,
    """    pub fn contains(&self, id: NodeId) -> bool {
        id.index() < self.nodes.len()
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {""",
    """    pub fn contains(&self, id: NodeId) -> bool {
        id.index() < self.nodes.len()
    }

    pub fn is_connected(&self, id: NodeId) -> bool {
        if !self.contains(id) {
            return false;
        }
        let mut cursor = Some(id);
        let mut steps = 0usize;
        while let Some(current) = cursor {
            if current == self.root {
                return true;
            }
            steps += 1;
            if steps > self.nodes.len() {
                return false;
            }
            cursor = self.nodes[current.index()].parent;
        }
        false
    }

    pub fn max_depth(&self) -> usize {
        let mut max_depth = 0usize;
        let mut stack = vec![(self.root, 1usize)];
        while let Some((node, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            if let Some(current) = self.node(node) {
                stack.extend(current.children.iter().copied().map(|child| (child, depth + 1)));
            }
        }
        max_depth
    }

    pub fn text_scalar_count(&self) -> usize {
        self.nodes
            .iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Text(text) => Some(text.chars().count()),
                NodeKind::Document | NodeKind::Element(_) => None,
            })
            .fold(0usize, usize::saturating_add)
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {""",
)
replace(
    dom,
    """    fn node_id_exposes_stable_index_without_public_construction() {
        let document = Document::new();
        assert_eq!(document.root().index(), 0);
        assert_eq!(document.root().to_string(), "0");
    }
}""",
    """    fn node_id_exposes_stable_index_without_public_construction() {
        let document = Document::new();
        assert_eq!(document.root().index(), 0);
        assert_eq!(document.root().to_string(), "0");
    }

    #[test]
    fn connectedness_depth_and_text_accounting_are_iterative_and_explicit() {
        let mut document = Document::new();
        let first = document.append_new(document.root(), element("div")).unwrap();
        let second = document.append_new(first, element("span")).unwrap();
        document
            .append_new(second, NodeKind::Text("Rarog".into()))
            .unwrap();
        let detached = document.create_node(element("section")).unwrap();

        assert!(document.is_connected(document.root()));
        assert!(document.is_connected(second));
        assert!(!document.is_connected(detached));
        assert_eq!(document.max_depth(), 4);
        assert_eq!(document.text_scalar_count(), 5);
    }
}""",
)

css = "crates/rarog-css/src/lib.rs"
replace(
    css,
    """                MutationKind::NodeCreated { node } => {
                    set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                }
                MutationKind::ChildAdded { parent, child } => {
                    set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    set.mark_ancestors(document, Some(*parent), DirtyFlags::LAYOUT_PAINT);
                    set.mark_structural_dependents(document, *parent, *child, dependencies);
                }""",
    """                MutationKind::NodeCreated { node } => {
                    if document.is_connected(*node) {
                        set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                }
                MutationKind::ChildAdded { parent, child } => {
                    if document.is_connected(*parent) {
                        set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                        set.mark_ancestors(document, Some(*parent), DirtyFlags::LAYOUT_PAINT);
                        set.mark_structural_dependents(document, *parent, *child, dependencies);
                    }
                }""",
)
replace(
    css,
    """                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    set.mark_ancestors(document, *old_parent, DirtyFlags::LAYOUT_PAINT);
                    set.mark_ancestors(document, *new_parent, DirtyFlags::LAYOUT_PAINT);
                    if dependencies.has_scope(SelectorDependencyScope::Descendants) {
                        mark_subtree(document, *child, &mut set, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                    if dependencies.has_scope(SelectorDependencyScope::FollowingSiblings) {""",
    """                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    let child_connected = document.is_connected(*child);
                    let old_connected = old_parent.is_some_and(|node| document.is_connected(node));
                    let new_connected = new_parent.is_some_and(|node| document.is_connected(node));
                    if child_connected {
                        set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                    if old_connected {
                        set.mark_ancestors(document, *old_parent, DirtyFlags::LAYOUT_PAINT);
                    }
                    if new_connected {
                        set.mark_ancestors(document, *new_parent, DirtyFlags::LAYOUT_PAINT);
                    }
                    if child_connected && dependencies.has_scope(SelectorDependencyScope::Descendants) {
                        mark_subtree(document, *child, &mut set, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                    if (old_connected || new_connected)
                        && dependencies.has_scope(SelectorDependencyScope::FollowingSiblings)
                    {""",
)
replace(
    css,
    """                MutationKind::Attribute { node, name } => {
                    if matches!(name.as_str(), "id" | "class" | "style") {""",
    """                MutationKind::Attribute { node, name } => {
                    if !document.is_connected(*node) {
                        continue;
                    }
                    if matches!(name.as_str(), "id" | "class" | "style") {""",
)
replace(
    css,
    """                MutationKind::CharacterData { node } => {
                    set.mark(*node, DirtyFlags::LAYOUT_PAINT);""",
    """                MutationKind::CharacterData { node } => {
                    if !document.is_connected(*node) {
                        continue;
                    }
                    set.mark(*node, DirtyFlags::LAYOUT_PAINT);""",
)
marker = """    #[test]
    fn non_finite_lengths_are_rejected() {"""
text = Path(css).read_text(encoding="utf-8")
if marker not in text:
    raise SystemExit("css test marker missing")
test = """    #[test]
    fn detached_mutations_do_not_dirty_the_connected_document() {
        let mut document = Document::new();
        let detached = document
            .create_node(NodeKind::Element(ElementData::html("div")))
            .unwrap();
        let generation = document.generation();
        document.set_attribute(detached, "class", "card").unwrap();

        let invalidation = InvalidationSet::from_document_since(&document, generation);
        assert!(invalidation.entries.is_empty());

        document.append_child(document.root(), detached).unwrap();
        let invalidation = InvalidationSet::from_document_since(&document, generation);
        assert!(invalidation.entries.contains_key(&detached));
    }

"""
Path(css).write_text(text.replace(marker, test + marker), encoding="utf-8")
