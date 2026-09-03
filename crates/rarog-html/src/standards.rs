use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, ExpandedName, QualName, parse_document};
use rarog_dom::{Atom, Document, ElementData, Namespace, NodeKind};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";

type Handle = Rc<AdapterNode>;

enum AdapterNodeData {
    Document,
    DocumentFragment,
    Element {
        name: QualName,
        attributes: RefCell<Vec<Attribute>>,
    },
    Text(RefCell<StrTendril>),
    Ignored,
}

struct AdapterNode {
    parent: RefCell<Option<Weak<AdapterNode>>>,
    children: RefCell<Vec<Handle>>,
    template_contents: RefCell<Option<Handle>>,
    data: AdapterNodeData,
}

impl AdapterNode {
    fn new(data: AdapterNodeData) -> Handle {
        Rc::new(Self {
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            template_contents: RefCell::new(None),
            data,
        })
    }
}

struct AdapterOutput {
    document: Handle,
    errors: Vec<(u64, String)>,
}

struct StandardsTreeSink {
    document: Handle,
    errors: RefCell<Vec<(u64, String)>>,
    current_line: Cell<u64>,
}

impl StandardsTreeSink {
    fn new() -> Self {
        Self {
            document: AdapterNode::new(AdapterNodeData::Document),
            errors: RefCell::new(Vec::new()),
            current_line: Cell::new(1),
        }
    }

    fn parent(node: &Handle) -> Option<Handle> {
        node.parent.borrow().as_ref().and_then(Weak::upgrade)
    }

    fn detach(node: &Handle) {
        let Some(parent) = Self::parent(node) else {
            return;
        };
        parent
            .children
            .borrow_mut()
            .retain(|candidate| !Rc::ptr_eq(candidate, node));
        *node.parent.borrow_mut() = None;
    }

    fn append_handle(parent: &Handle, child: Handle) {
        Self::detach(&child);
        if let AdapterNodeData::Text(text) = &child.data {
            if let Some(last) = parent.children.borrow().last().cloned() {
                if let AdapterNodeData::Text(last_text) = &last.data {
                    last_text.borrow_mut().push_tendril(&text.borrow());
                    return;
                }
            }
        }
        *child.parent.borrow_mut() = Some(Rc::downgrade(parent));
        parent.children.borrow_mut().push(child);
    }

    fn append_text(parent: &Handle, text: StrTendril) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = parent.children.borrow().last().cloned() {
            if let AdapterNodeData::Text(last_text) = &last.data {
                last_text.borrow_mut().push_tendril(&text);
                return;
            }
        }
        let child = AdapterNode::new(AdapterNodeData::Text(RefCell::new(text)));
        *child.parent.borrow_mut() = Some(Rc::downgrade(parent));
        parent.children.borrow_mut().push(child);
    }

    fn insert_before(sibling: &Handle, child: NodeOrText<Handle>) {
        let Some(parent) = Self::parent(sibling) else {
            return;
        };
        match child {
            NodeOrText::AppendNode(node) => {
                Self::detach(&node);
                let mut children = parent.children.borrow_mut();
                let index = children
                    .iter()
                    .position(|candidate| Rc::ptr_eq(candidate, sibling))
                    .expect("tree builder sibling belongs to its parent");
                if let AdapterNodeData::Text(text) = &node.data {
                    if index > 0 {
                        if let AdapterNodeData::Text(previous) = &children[index - 1].data {
                            previous.borrow_mut().push_tendril(&text.borrow());
                            return;
                        }
                    }
                }
                *node.parent.borrow_mut() = Some(Rc::downgrade(&parent));
                children.insert(index, node);
            }
            NodeOrText::AppendText(text) => {
                let mut children = parent.children.borrow_mut();
                let index = children
                    .iter()
                    .position(|candidate| Rc::ptr_eq(candidate, sibling))
                    .expect("tree builder sibling belongs to its parent");
                if index > 0 {
                    if let AdapterNodeData::Text(previous) = &children[index - 1].data {
                        previous.borrow_mut().push_tendril(&text);
                        return;
                    }
                }
                let node = AdapterNode::new(AdapterNodeData::Text(RefCell::new(text)));
                *node.parent.borrow_mut() = Some(Rc::downgrade(&parent));
                children.insert(index, node);
            }
        }
    }
}

impl TreeSink for StandardsTreeSink {
    type Handle = Handle;
    type Output = AdapterOutput;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> Self::Output {
        AdapterOutput {
            document: self.document,
            errors: self.errors.into_inner(),
        }
    }

    fn parse_error(&self, message: Cow<'static, str>) {
        self.errors
            .borrow_mut()
            .push((self.current_line.get(), message.into_owned()));
    }

    fn get_document(&self) -> Self::Handle {
        self.document.clone()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        match &target.data {
            AdapterNodeData::Element { name, .. } => name.expanded(),
            _ => panic!("tree builder requested an element name from a non-element"),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attributes: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let is_template = name.ns.as_ref() == HTML_NAMESPACE && name.local.as_ref() == "template";
        let node = AdapterNode::new(AdapterNodeData::Element {
            name,
            attributes: RefCell::new(attributes),
        });
        if is_template {
            *node.template_contents.borrow_mut() =
                Some(AdapterNode::new(AdapterNodeData::DocumentFragment));
        }
        node
    }

    fn create_comment(&self, _text: StrTendril) -> Self::Handle {
        AdapterNode::new(AdapterNodeData::Ignored)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        AdapterNode::new(AdapterNodeData::Ignored)
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(node) => Self::append_handle(parent, node),
            NodeOrText::AppendText(text) => Self::append_text(parent, text),
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        previous_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if Self::parent(element).is_some() {
            Self::insert_before(element, child);
        } else {
            self.append(previous_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        Self::append_handle(&self.document, AdapterNode::new(AdapterNodeData::Ignored));
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        target
            .template_contents
            .borrow()
            .as_ref()
            .expect("template elements have template contents")
            .clone()
    }

    fn same_node(&self, left: &Self::Handle, right: &Self::Handle) -> bool {
        Rc::ptr_eq(left, right)
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        Self::insert_before(sibling, new_node);
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attributes: Vec<Attribute>) {
        let AdapterNodeData::Element {
            attributes: current,
            ..
        } = &target.data
        else {
            return;
        };
        let mut current = current.borrow_mut();
        for attribute in attributes {
            if current
                .iter()
                .all(|existing| existing.name != attribute.name)
            {
                current.push(attribute);
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        Self::detach(target);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let children = {
            let mut children = node.children.borrow_mut();
            std::mem::take(&mut *children)
        };
        for child in children {
            *child.parent.borrow_mut() = None;
            Self::append_handle(new_parent, child);
        }
    }

    fn set_current_line(&self, line_number: u64) {
        self.current_line.set(line_number);
    }
}

pub(crate) struct StandardsParseOutput {
    pub document: Document,
    pub errors: Vec<(u64, String)>,
}

pub(crate) fn parse(input: &str) -> StandardsParseOutput {
    let parsed = parse_document(StandardsTreeSink::new(), Default::default())
        .one(StrTendril::from_slice(input));
    StandardsParseOutput {
        document: convert_document(&parsed.document),
        errors: parsed.errors,
    }
}

fn convert_document(root: &Handle) -> Document {
    let mut document = Document::new();
    let root_id = document.root();
    let mut stack = root
        .children
        .borrow()
        .iter()
        .rev()
        .cloned()
        .map(|node| (node, root_id))
        .collect::<Vec<_>>();

    while let Some((node, parent)) = stack.pop() {
        match &node.data {
            AdapterNodeData::Element { name, attributes } => {
                let attributes = attributes
                    .borrow()
                    .iter()
                    .map(|attribute| (attribute_name(attribute), attribute.value.to_string()))
                    .collect::<BTreeMap<_, _>>();
                let element = ElementData::new(namespace(name.ns.as_ref()), name.local.to_string())
                    .with_attributes(attributes);
                let id = document
                    .append_new(parent, NodeKind::Element(element))
                    .expect("standards adapter emits a valid tree");
                stack.extend(
                    node.children
                        .borrow()
                        .iter()
                        .rev()
                        .cloned()
                        .map(|child| (child, id)),
                );
            }
            AdapterNodeData::Text(text) => {
                let text = text.borrow().to_string();
                if !text.is_empty() {
                    document
                        .append_new(parent, NodeKind::Text(text))
                        .expect("standards adapter emits text under valid parents");
                }
            }
            AdapterNodeData::Document
            | AdapterNodeData::DocumentFragment
            | AdapterNodeData::Ignored => {
                stack.extend(
                    node.children
                        .borrow()
                        .iter()
                        .rev()
                        .cloned()
                        .map(|child| (child, parent)),
                );
            }
        }
    }

    document
}

fn namespace(value: &str) -> Namespace {
    match value {
        HTML_NAMESPACE => Namespace::Html,
        SVG_NAMESPACE => Namespace::Svg,
        MATHML_NAMESPACE => Namespace::MathMl,
        other => Namespace::Other(Atom::from(other)),
    }
}

fn attribute_name(attribute: &Attribute) -> String {
    match attribute.name.prefix.as_ref() {
        Some(prefix) => format!("{}:{}", prefix, attribute.name.local),
        None => attribute.name.local.to_string(),
    }
}
