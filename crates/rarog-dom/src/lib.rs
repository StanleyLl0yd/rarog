use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub type NodeId = usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Element(ElementData),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutationKind {
    NodeCreated {
        node: NodeId,
    },
    ChildAdded {
        parent: NodeId,
        child: NodeId,
    },
    Reparented {
        child: NodeId,
        old_parent: Option<NodeId>,
        new_parent: Option<NodeId>,
    },
    Attribute {
        node: NodeId,
        name: String,
    },
    CharacterData {
        node: NodeId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationRecord {
    pub generation: u64,
    pub kind: MutationKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutationError {
    InvalidNode(NodeId),
    CannotCreateDocumentNode,
    CannotReparentRoot,
    CannotAppendToText(NodeId),
    NotElement(NodeId),
    NotText(NodeId),
    WouldCreateCycle { parent: NodeId, child: NodeId },
}

impl fmt::Display for MutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(node) => write!(formatter, "invalid DOM node id {node}"),
            Self::CannotCreateDocumentNode => {
                formatter.write_str("the document node can only be created by Document::new")
            }
            Self::CannotReparentRoot => {
                formatter.write_str("the document root cannot be reparented")
            }
            Self::CannotAppendToText(node) => {
                write!(formatter, "text node {node} cannot have children")
            }
            Self::NotElement(node) => write!(formatter, "node {node} is not an element"),
            Self::NotText(node) => write!(formatter, "node {node} is not a text node"),
            Self::WouldCreateCycle { parent, child } => {
                write!(
                    formatter,
                    "appending node {child} to {parent} would create a cycle"
                )
            }
        }
    }
}

impl Error for MutationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvariantError {
    RootHasParent,
    RootIsNotDocument,
    NonRootDocument(NodeId),
    InvalidParent {
        node: NodeId,
        parent: NodeId,
    },
    MissingChildLink {
        node: NodeId,
        parent: NodeId,
    },
    InvalidChild {
        parent: NodeId,
        child: NodeId,
    },
    WrongParent {
        parent: NodeId,
        child: NodeId,
        actual: Option<NodeId>,
    },
    DuplicateChild {
        parent: NodeId,
        child: NodeId,
    },
    Cycle(NodeId),
}

#[derive(Clone, Debug)]
pub struct Document {
    nodes: Vec<Node>,
    root: NodeId,
    generation: u64,
    mutations: Vec<MutationRecord>,
}

impl Document {
    pub fn new() -> Self {
        let root = Node {
            kind: NodeKind::Document,
            parent: None,
            children: Vec::new(),
        };
        Self {
            nodes: vec![root],
            root: 0,
            generation: 0,
            mutations: Vec::new(),
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn contains(&self, id: NodeId) -> bool {
        id < self.nodes.len()
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn try_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id].children
    }

    pub fn mutation_records_since(
        &self,
        generation: u64,
    ) -> impl Iterator<Item = &MutationRecord> {
        self.mutations
            .iter()
            .filter(move |record| record.generation > generation)
    }

    pub fn create_node(&mut self, kind: NodeKind) -> Result<NodeId, MutationError> {
        self.ensure_creatable_kind(&kind)?;
        let id = self.allocate_node(kind, None);
        self.record_mutation(MutationKind::NodeCreated { node: id });
        Ok(id)
    }

    pub fn append_new(&mut self, parent: NodeId, kind: NodeKind) -> Result<NodeId, MutationError> {
        self.ensure_can_have_children(parent)?;
        self.ensure_creatable_kind(&kind)?;

        let id = self.allocate_node(kind, Some(parent));
        self.nodes[parent].children.push(id);
        self.record_mutation(MutationKind::ChildAdded { parent, child: id });
        Ok(id)
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), MutationError> {
        self.ensure_can_have_children(parent)?;
        self.ensure_node(child)?;

        if child == self.root {
            return Err(MutationError::CannotReparentRoot);
        }

        if parent == child || self.is_ancestor(child, parent) {
            return Err(MutationError::WouldCreateCycle { parent, child });
        }

        if self.nodes[child].parent == Some(parent) && self.nodes[parent].children.contains(&child)
        {
            return Ok(());
        }

        let old_parent = self.nodes[child].parent;
        if let Some(old_parent) = old_parent {
            self.nodes[old_parent]
                .children
                .retain(|candidate| *candidate != child);
        }

        self.nodes[child].parent = Some(parent);
        if !self.nodes[parent].children.contains(&child) {
            self.nodes[parent].children.push(child);
        }
        self.record_mutation(MutationKind::Reparented {
            child,
            old_parent,
            new_parent: Some(parent),
        });
        Ok(())
    }

    pub fn detach(&mut self, child: NodeId) -> Result<(), MutationError> {
        self.ensure_node(child)?;
        if child == self.root {
            return Err(MutationError::CannotReparentRoot);
        }

        let Some(parent) = self.nodes[child].parent else {
            return Ok(());
        };

        self.nodes[parent]
            .children
            .retain(|candidate| *candidate != child);
        self.nodes[child].parent = None;
        self.record_mutation(MutationKind::Reparented {
            child,
            old_parent: Some(parent),
            new_parent: None,
        });
        Ok(())
    }

    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), MutationError> {
        self.ensure_node(node)?;
        let name = name.into();
        let value = value.into();

        let NodeKind::Element(element) = &mut self.nodes[node].kind else {
            return Err(MutationError::NotElement(node));
        };

        if element.attributes.get(&name) == Some(&value) {
            return Ok(());
        }

        element.attributes.insert(name.clone(), value);
        self.record_mutation(MutationKind::Attribute { node, name });
        Ok(())
    }

    pub fn remove_attribute(
        &mut self,
        node: NodeId,
        name: &str,
    ) -> Result<Option<String>, MutationError> {
        self.ensure_node(node)?;
        let NodeKind::Element(element) = &mut self.nodes[node].kind else {
            return Err(MutationError::NotElement(node));
        };

        let removed = element.attributes.remove(name);
        if removed.is_some() {
            self.record_mutation(MutationKind::Attribute {
                node,
                name: name.to_string(),
            });
        }
        Ok(removed)
    }

    pub fn set_text(
        &mut self,
        node: NodeId,
        value: impl Into<String>,
    ) -> Result<(), MutationError> {
        self.ensure_node(node)?;
        let value = value.into();
        let NodeKind::Text(text) = &mut self.nodes[node].kind else {
            return Err(MutationError::NotText(node));
        };

        if text == &value {
            return Ok(());
        }

        *text = value;
        self.record_mutation(MutationKind::CharacterData { node });
        Ok(())
    }

    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        for (id, node) in self.nodes.iter().enumerate() {
            let parent = node
                .parent
                .map(|parent| parent.to_string())
                .unwrap_or_else(|| "-".to_string());
            let children = node
                .children
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let kind = match &node.kind {
                NodeKind::Document => "document".to_string(),
                NodeKind::Text(text) => format!("text:{}", escape_snapshot(text)),
                NodeKind::Element(element) => {
                    let attributes = element
                        .attributes
                        .iter()
                        .map(|(name, value)| {
                            format!("{}={}", escape_snapshot(name), escape_snapshot(value))
                        })
                        .collect::<Vec<_>>()
                        .join(";");
                    format!(
                        "element:{}[{}]",
                        escape_snapshot(&element.tag_name),
                        attributes
                    )
                }
            };
            output.push_str(&format!(
                "{id}|{kind}|parent={parent}|children={children}\n"
            ));
        }
        output
    }

    pub fn validate_invariants(&self) -> Result<(), InvariantError> {
        if self.nodes[self.root].parent.is_some() {
            return Err(InvariantError::RootHasParent);
        }
        if !matches!(self.nodes[self.root].kind, NodeKind::Document) {
            return Err(InvariantError::RootIsNotDocument);
        }

        for (id, node) in self.nodes.iter().enumerate() {
            if id != self.root && matches!(node.kind, NodeKind::Document) {
                return Err(InvariantError::NonRootDocument(id));
            }

            if let Some(parent) = node.parent {
                if !self.contains(parent) {
                    return Err(InvariantError::InvalidParent { node: id, parent });
                }
                if !self.nodes[parent].children.contains(&id) {
                    return Err(InvariantError::MissingChildLink { node: id, parent });
                }
            }

            let mut seen_children = BTreeSet::new();
            for child in &node.children {
                if !self.contains(*child) {
                    return Err(InvariantError::InvalidChild {
                        parent: id,
                        child: *child,
                    });
                }
                if !seen_children.insert(*child) {
                    return Err(InvariantError::DuplicateChild {
                        parent: id,
                        child: *child,
                    });
                }
                if self.nodes[*child].parent != Some(id) {
                    return Err(InvariantError::WrongParent {
                        parent: id,
                        child: *child,
                        actual: self.nodes[*child].parent,
                    });
                }
            }

            let mut seen_ancestors = BTreeSet::new();
            let mut cursor = Some(id);
            while let Some(current) = cursor {
                if !seen_ancestors.insert(current) {
                    return Err(InvariantError::Cycle(id));
                }
                cursor = self.nodes[current].parent;
            }
        }

        Ok(())
    }

    fn ensure_node(&self, node: NodeId) -> Result<(), MutationError> {
        if self.contains(node) {
            Ok(())
        } else {
            Err(MutationError::InvalidNode(node))
        }
    }

    fn ensure_can_have_children(&self, node: NodeId) -> Result<(), MutationError> {
        self.ensure_node(node)?;
        if matches!(self.nodes[node].kind, NodeKind::Text(_)) {
            Err(MutationError::CannotAppendToText(node))
        } else {
            Ok(())
        }
    }

    fn ensure_creatable_kind(&self, kind: &NodeKind) -> Result<(), MutationError> {
        if matches!(kind, NodeKind::Document) {
            Err(MutationError::CannotCreateDocumentNode)
        } else {
            Ok(())
        }
    }

    fn allocate_node(&mut self, kind: NodeKind, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            kind,
            parent,
            children: Vec::new(),
        });
        id
    }

    fn is_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut cursor = Some(node);
        while let Some(current) = cursor {
            if current == ancestor {
                return true;
            }
            cursor = self.nodes[current].parent;
        }
        false
    }

    fn record_mutation(&mut self, kind: MutationKind) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("DOM generation counter overflow");
        self.mutations.push(MutationRecord {
            generation: self.generation,
            kind,
        });
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

fn escape_snapshot(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(name: &str) -> NodeKind {
        NodeKind::Element(ElementData {
            tag_name: name.into(),
            attributes: BTreeMap::new(),
        })
    }

    #[test]
    fn append_preserves_parent_child_relationship() {
        let mut doc = Document::new();
        let child = doc
            .append_new(doc.root(), NodeKind::Text("hello".into()))
            .unwrap();

        assert_eq!(doc.node(child).parent, Some(doc.root()));
        assert_eq!(doc.children(doc.root()), &[child]);
        assert_eq!(doc.validate_invariants(), Ok(()));
    }

    #[test]
    fn reparent_updates_both_sides_of_the_relationship() {
        let mut doc = Document::new();
        let first = doc.append_new(doc.root(), element("div")).unwrap();
        let second = doc.append_new(doc.root(), element("section")).unwrap();
        let child = doc.append_new(first, element("span")).unwrap();

        doc.append_child(second, child).unwrap();

        assert!(doc.children(first).is_empty());
        assert_eq!(doc.children(second), &[child]);
        assert_eq!(doc.node(child).parent, Some(second));
        assert_eq!(doc.validate_invariants(), Ok(()));
    }

    #[test]
    fn cycle_is_rejected_without_mutating_the_tree() {
        let mut doc = Document::new();
        let parent = doc.append_new(doc.root(), element("div")).unwrap();
        let child = doc.append_new(parent, element("span")).unwrap();
        let generation = doc.generation();

        let result = doc.append_child(child, parent);

        assert_eq!(
            result,
            Err(MutationError::WouldCreateCycle {
                parent: child,
                child: parent,
            })
        );
        assert_eq!(doc.generation(), generation);
        assert_eq!(doc.validate_invariants(), Ok(()));
    }

    #[test]
    fn text_nodes_cannot_become_parents() {
        let mut doc = Document::new();
        let text = doc
            .append_new(doc.root(), NodeKind::Text("hello".into()))
            .unwrap();
        let detached = doc.create_node(element("span")).unwrap();

        assert_eq!(
            doc.append_child(text, detached),
            Err(MutationError::CannotAppendToText(text))
        );
        assert_eq!(doc.node(detached).parent, None);
    }

    #[test]
    fn attribute_and_text_mutations_advance_generation_only_when_changed() {
        let mut doc = Document::new();
        let element = doc.append_new(doc.root(), element("div")).unwrap();
        let text = doc
            .append_new(element, NodeKind::Text("before".into()))
            .unwrap();
        let before = doc.generation();

        doc.set_attribute(element, "class", "card").unwrap();
        doc.set_text(text, "after").unwrap();
        let after_changes = doc.generation();

        doc.set_attribute(element, "class", "card").unwrap();
        doc.set_text(text, "after").unwrap();

        assert_eq!(after_changes, before + 2);
        assert_eq!(doc.generation(), after_changes);
        assert_eq!(doc.validate_invariants(), Ok(()));
    }

    #[test]
    fn mutation_log_is_generation_ordered_and_queryable() {
        let mut doc = Document::new();
        let element = doc.append_new(doc.root(), element("div")).unwrap();
        let generation = doc.generation();
        doc.set_attribute(element, "class", "card").unwrap();
        doc.set_attribute(element, "id", "hero").unwrap();

        let records = doc
            .mutation_records_since(generation)
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(records.len(), 2);
        assert!(records[0].generation < records[1].generation);
        assert_eq!(
            records[0].kind,
            MutationKind::Attribute {
                node: element,
                name: "class".into(),
            }
        );
    }

    #[test]
    fn snapshot_is_stable_for_the_same_arena() {
        let mut doc = Document::new();
        let element = doc.append_new(doc.root(), element("div")).unwrap();
        doc.set_attribute(element, "id", "hero").unwrap();
        doc.append_new(element, NodeKind::Text("hello".into()))
            .unwrap();

        assert_eq!(doc.snapshot(), doc.snapshot());
        assert!(doc.snapshot().contains("element:div[id=hero]"));
    }

    #[test]
    fn document_root_cannot_be_reparented_or_detached() {
        let mut doc = Document::new();
        let parent = doc.append_new(doc.root(), element("div")).unwrap();

        assert_eq!(
            doc.append_child(parent, doc.root()),
            Err(MutationError::CannotReparentRoot)
        );
        assert_eq!(
            doc.detach(doc.root()),
            Err(MutationError::CannotReparentRoot)
        );
    }
}
