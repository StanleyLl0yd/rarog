from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker: {label}")
    return text.replace(old, new, 1)


dom = Path("crates/rarog-dom/src/lib.rs")
text = dom.read_text()
text = replace_once(
    text,
    "use std::collections::{BTreeMap, BTreeSet};\nuse std::error::Error;\nuse std::fmt;\n",
    "use std::borrow::Borrow;\nuse std::collections::{BTreeMap, BTreeSet};\nuse std::error::Error;\nuse std::fmt;\nuse std::ops::Deref;\nuse std::sync::Arc;\n",
    "DOM imports",
)
text = replace_once(
    text,
    "pub type NodeId = usize;\n\n",
    '''pub type NodeId = usize;\n\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct Atom(Arc<str>);\n\nimpl Atom {\n    pub fn new(value: impl AsRef<str>) -> Self {\n        Self(Arc::from(value.as_ref()))\n    }\n\n    pub fn as_str(&self) -> &str {\n        &self.0\n    }\n\n    pub fn ptr_eq(&self, other: &Self) -> bool {\n        Arc::ptr_eq(&self.0, &other.0)\n    }\n}\n\nimpl From<&str> for Atom {\n    fn from(value: &str) -> Self {\n        Self::new(value)\n    }\n}\n\nimpl From<String> for Atom {\n    fn from(value: String) -> Self {\n        Self(Arc::from(value))\n    }\n}\n\nimpl AsRef<str> for Atom {\n    fn as_ref(&self) -> &str {\n        self.as_str()\n    }\n}\n\nimpl Borrow<str> for Atom {\n    fn borrow(&self) -> &str {\n        self.as_str()\n    }\n}\n\nimpl Deref for Atom {\n    type Target = str;\n\n    fn deref(&self) -> &Self::Target {\n        self.as_str()\n    }\n}\n\nimpl fmt::Display for Atom {\n    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {\n        formatter.write_str(self.as_str())\n    }\n}\n\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub enum Namespace {\n    Html,\n    Svg,\n    MathMl,\n    Other(Atom),\n}\n\nimpl Namespace {\n    pub fn as_str(&self) -> &str {\n        match self {\n            Self::Html => "html",\n            Self::Svg => "svg",\n            Self::MathMl => "mathml",\n            Self::Other(namespace) => namespace.as_str(),\n        }\n    }\n}\n\n''',
    "NodeId",
)
text = replace_once(
    text,
    '''#[derive(Clone, Debug, PartialEq, Eq)]\npub struct ElementData {\n    pub tag_name: String,\n    pub attributes: BTreeMap<String, String>,\n}\n''',
    '''#[derive(Clone, Debug, PartialEq, Eq)]\npub struct ElementData {\n    pub namespace: Namespace,\n    pub tag_name: Atom,\n    pub attributes: BTreeMap<String, String>,\n}\n\nimpl ElementData {\n    pub fn new(namespace: Namespace, tag_name: impl Into<Atom>) -> Self {\n        Self {\n            namespace,\n            tag_name: tag_name.into(),\n            attributes: BTreeMap::new(),\n        }\n    }\n\n    pub fn html(tag_name: impl Into<Atom>) -> Self {\n        Self::new(Namespace::Html, tag_name)\n    }\n\n    pub fn with_attributes(mut self, attributes: BTreeMap<String, String>) -> Self {\n        self.attributes = attributes;\n        self\n    }\n}\n''',
    "ElementData",
)
text = replace_once(
    text,
    '''                    format!(\n                        "element:{}[{}]",\n                        escape_snapshot(&element.tag_name),\n                        attributes\n                    )\n''',
    '''                    let name = if element.namespace == Namespace::Html {\n                        escape_snapshot(element.tag_name.as_str())\n                    } else {\n                        format!(\n                            "{}:{}",\n                            escape_snapshot(element.namespace.as_str()),\n                            escape_snapshot(element.tag_name.as_str())\n                        )\n                    };\n                    format!("element:{name}[{attributes}]")\n''',
    "DOM snapshot",
)
helper = '''    fn element(name: &str) -> NodeKind {\n        NodeKind::Element(ElementData {\n            tag_name: name.into(),\n            attributes: BTreeMap::new(),\n        })\n    }\n'''
if text.count(helper) != 2:
    raise SystemExit(f"expected two DOM element helpers, found {text.count(helper)}")
text = text.replace(
    helper,
    '''    fn element(name: &str) -> NodeKind {\n        NodeKind::Element(ElementData::html(name))\n    }\n''',
)
text = text.replace("    use std::collections::BTreeMap;\n\n    fn element(name: &str) -> NodeKind {", "    fn element(name: &str) -> NodeKind {", 1)
test_anchor = '''    fn document_root_cannot_be_reparented_or_detached() {\n        let mut doc = Document::new();\n        let parent = doc.append_new(doc.root(), element("div")).unwrap();\n\n        assert_eq!(\n            doc.append_child(parent, doc.root()),\n            Err(MutationError::CannotReparentRoot)\n        );\n        assert_eq!(\n            doc.detach(doc.root()),\n            Err(MutationError::CannotReparentRoot)\n        );\n    }\n'''
text = replace_once(
    text,
    test_anchor,
    test_anchor
    + '''\n    #[test]\n    fn element_namespace_and_atom_identity_are_explicit() {\n        let mut doc = Document::new();\n        let name = Atom::from("circle");\n        let cloned = name.clone();\n        assert!(name.ptr_eq(&cloned));\n\n        let node = doc\n            .append_new(\n                doc.root(),\n                NodeKind::Element(ElementData::new(Namespace::Svg, name)),\n            )\n            .unwrap();\n        let NodeKind::Element(element) = &doc.node(node).kind else {\n            panic!("expected element");\n        };\n        assert_eq!(element.namespace, Namespace::Svg);\n        assert_eq!(element.tag_name.as_str(), "circle");\n        assert!(doc.snapshot().contains("element:svg:circle[]"));\n    }\n''',
    "DOM namespace test anchor",
)
dom.write_text(text)

html = Path("crates/rarog-html/src/lib.rs")
text = html.read_text()
text = replace_once(
    text,
    '''                                NodeKind::Element(ElementData {\n                                    tag_name: tag,\n                                    attributes: attrs,\n                                }),\n''',
    '''                                NodeKind::Element(ElementData::html(tag).with_attributes(attrs)),\n''',
    "HTML element construction",
)
text = replace_once(
    text,
    '''        assert_eq!(doc.validate_invariants(), Ok(()));\n        assert!(doc.generation() > 0);\n''',
    '''        assert_eq!(doc.validate_invariants(), Ok(()));\n        assert!(doc.generation() > 0);\n        let html = doc.children(doc.root())[0];\n        let NodeKind::Element(element) = &doc.node(html).kind else {\n            panic!("expected html element");\n        };\n        assert_eq!(element.namespace, rarog_dom::Namespace::Html);\n        assert_eq!(element.tag_name.as_str(), "html");\n''',
    "HTML namespace test",
)
html.write_text(text)

css = Path("crates/rarog-css/src/lib.rs")
text = css.read_text()
text = text.replace("if &element.tag_name != tag {", "if element.tag_name.as_str() != tag {")
text = text.replace('if element.tag_name == "style" {', 'if element.tag_name.as_str() == "style" {')
text = replace_once(
    text,
    '''                NodeKind::Element(ElementData {\n                    tag_name: tag.into(),\n                    attributes: attrs,\n                }),\n''',
    '''                NodeKind::Element(ElementData::html(tag).with_attributes(attrs)),\n''',
    "CSS test helper",
)
text = replace_once(
    text,
    '''                NodeKind::Element(ElementData {\n                    tag_name: "style".into(),\n                    attributes: BTreeMap::new(),\n                }),\n''',
    '''                NodeKind::Element(ElementData::html("style")),\n''',
    "CSS style element test",
)
text = replace_once(
    text,
    '''                NodeKind::Element(ElementData {\n                    tag_name: "div".into(),\n                    attributes,\n                }),\n''',
    '''                NodeKind::Element(ElementData::html("div").with_attributes(attributes)),\n''',
    "CSS target element test",
)
css.write_text(text)

layout = Path("crates/rarog-layout/src/lib.rs")
text = layout.read_text()
text = replace_once(
    text,
    '''        NodeKind::Element(ElementData {\n            tag_name: name.into(),\n            attributes,\n        })\n''',
    '''        NodeKind::Element(ElementData::html(name).with_attributes(attributes))\n''',
    "layout test helper",
)
text = replace_once(
    text,
    '''            NodeKind::Element(ElementData {\n                tag_name: "div".into(),\n                attributes,\n            }),\n''',
    '''            NodeKind::Element(ElementData::html("div").with_attributes(attributes)),\n''',
    "layout stylesheet target",
)
layout.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
text = architecture.read_text()
anchor = "The DOM does not know which selectors, layout nodes or paint items depend on a mutation.\n\n"
addition = '''The DOM does not know which selectors, layout nodes or paint items depend on a mutation.\n\n### Element names, namespaces and atoms\n\nR0 stores an explicit `Namespace` on every `ElementData` and represents the local element name with an immutable `Atom`. The bootstrap HTML parser assigns `Namespace::Html` only; SVG/MathML tree-building and namespace switching remain standards-parser work. Non-HTML namespaces can already be represented by the DOM without encoding namespace state into tag-name strings.\n\n`Atom` is the semantic boundary for frequently repeated engine-owned names. Its R0 storage is a cheap cloneable `Arc<str>` handle, not a process-global interning table. The long-term strategy is document/process-scoped canonical interning behind the same boundary once measurements justify it. Text-node contents and attribute values remain ordinary owned strings. A process-global immortal string table is intentionally rejected because it conflicts with bounded lifetimes, site isolation and explicit resource budgets. See ADR-0024.\n\n'''
text = replace_once(text, anchor, addition, "architecture DOM anchor")
architecture.write_text(text)

backlog = Path("docs/R0-BACKLOG.md")
text = backlog.read_text()
text = text.replace("- [ ] element namespace representation", "- [x] element namespace representation")
text = text.replace("- [ ] interned atom/string strategy ADR", "- [x] interned atom/string strategy ADR")
backlog.write_text(text)

Path("docs/adr/ADR-0024-element-namespaces-and-atoms.md").write_text(
    '''# ADR-0024: Element namespaces and atom strategy\n\n## Status\n\nAccepted.\n\n## Context\n\nR0 originally stored an element name as a plain `String` and implicitly treated every element as HTML. That is sufficient for the first bootstrap fixture but is the wrong ownership boundary for a Web DOM that will later represent HTML, SVG, MathML and namespaced content. Frequently repeated engine names also need a stable semantic type before selector, parser and WebIDL work make string allocation policy harder to change.\n\n## Decision\n\nEvery `ElementData` stores an explicit `Namespace` plus an `Atom` local name. R0 defines built-in HTML, SVG and MathML namespace variants and an `Other(Atom)` escape hatch without claiming namespace-aware HTML parsing. The bootstrap HTML parser creates HTML elements only.\n\n`Atom` uses immutable shared `Arc<str>` storage in R0. Clones share the same allocation, but independently-created equal atoms are not required to be pointer-identical. If measurements justify canonical interning, it will be document/process scoped and implemented behind the atom boundary rather than through a process-global immortal table.\n\nText-node data and attribute values are not atomized. Attribute-name atomization and namespace-aware attributes may be introduced later when the standards parser and selector model require them.\n\n## Consequences\n\n- DOM element namespace is no longer implicit in a tag-name string.\n- Existing HTML snapshots keep their previous spelling; non-HTML snapshots include a namespace prefix for deterministic diagnostics.\n- Atom cloning is cheap and gives later parser/style code a replaceable string-storage boundary.\n- There is no global interner lifetime shared across sites or processes.\n- This ADR does not implement HTML namespace switching, foreign-content parsing, namespaced CSS selectors or SVG/MathML layout semantics.\n'''
)
