from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker: {label}")
    return text.replace(old, new, 1)


css = Path("crates/rarog-css/src/lib.rs")
text = css.read_text()

text = replace_once(
    text,
    '''#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectorInvalidationKey {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: BTreeSet<String>,
}
''',
    '''#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectorInvalidationKey {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: BTreeSet<String>,
}

impl SelectorInvalidationKey {
    pub fn depends_on_attribute(&self, name: &str) -> bool {
        match name {
            "id" => self.id.is_some(),
            "class" => !self.classes.is_empty(),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectorDependencyScope {
    Descendants,
    FollowingSiblings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorDependency {
    pub trigger: SelectorInvalidationKey,
    pub scope: SelectorDependencyScope,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectorInvalidationDependencies {
    entries: Vec<SelectorDependency>,
}

impl SelectorInvalidationDependencies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, dependency: SelectorDependency) {
        if !self.entries.contains(&dependency) {
            self.entries.push(dependency);
        }
    }

    pub fn entries(&self) -> &[SelectorDependency] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has_scope(&self, scope: SelectorDependencyScope) -> bool {
        self.entries.iter().any(|dependency| dependency.scope == scope)
    }

    fn for_attribute<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a SelectorDependency> + 'a {
        self.entries
            .iter()
            .filter(move |dependency| dependency.trigger.depends_on_attribute(name))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StyleSharingKey {
    pub namespace: String,
    pub tag_name: String,
    pub id: Option<String>,
    pub classes: BTreeSet<String>,
    pub inline_style: Option<String>,
}

pub fn style_sharing_key(document: &Document, node: NodeId) -> Option<StyleSharingKey> {
    let NodeKind::Element(element) = &document.node(node).kind else {
        return None;
    };
    Some(StyleSharingKey {
        namespace: element.namespace.as_str().to_owned(),
        tag_name: element.tag_name.as_str().to_owned(),
        id: element.attributes.get("id").cloned(),
        classes: element
            .attributes
            .get("class")
            .map(|value| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default(),
        inline_style: element.attributes.get("style").cloned(),
    })
}
''',
    "selector invalidation key",
)

text = replace_once(
    text,
    '''pub struct StyleRule {
    pub selector: Selector,
    pub specificity: Specificity,
    pub declarations: Vec<Declaration>,
    pub source_order: u32,
}
''',
    '''pub struct StyleRule {
    pub selector: Selector,
    pub specificity: Specificity,
    pub declarations: Vec<Declaration>,
    pub source_order: u32,
    pub dependencies: Vec<SelectorDependency>,
}
''',
    "style rule",
)

text = replace_once(
    text,
    '''                        rules.push(StyleRule {
                            specificity: selector.specificity(),
                            selector,
                            declarations: declarations.clone(),
                            source_order,
                        });
''',
    '''                        rules.push(StyleRule {
                            specificity: selector.specificity(),
                            selector,
                            declarations: declarations.clone(),
                            source_order,
                            dependencies: Vec::new(),
                        });
''',
    "stylesheet rule construction",
)

text = replace_once(
    text,
    '''    pub fn snapshot(&self) -> String {
        let mut output = String::new();
''',
    '''    pub fn invalidation_dependencies(&self) -> SelectorInvalidationDependencies {
        let mut dependencies = SelectorInvalidationDependencies::new();
        for stylesheet in &self.stylesheets {
            for rule in &stylesheet.rules {
                for dependency in &rule.dependencies {
                    dependencies.register(dependency.clone());
                }
            }
        }
        dependencies
    }

    pub fn local_style_sharing_safe(&self) -> bool {
        self.stylesheets
            .iter()
            .flat_map(|stylesheet| &stylesheet.rules)
            .all(|rule| rule.dependencies.is_empty())
    }

    pub fn snapshot(&self) -> String {
        let mut output = String::new();
''',
    "style set methods",
)

old_invalidation = '''impl InvalidationSet {
    pub fn from_document_since(document: &Document, generation: u64) -> Self {
        let mut set = Self {
            entries: BTreeMap::new(),
            through_generation: document.generation(),
        };

        for record in document.mutation_records_since(generation) {
            match &record.kind {
                MutationKind::NodeCreated { node } => {
                    set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                }
                MutationKind::ChildAdded { parent, child } => {
                    set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    set.mark_ancestors(document, Some(*parent), DirtyFlags::LAYOUT_PAINT);
                }
                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    set.mark_ancestors(document, *old_parent, DirtyFlags::LAYOUT_PAINT);
                    set.mark_ancestors(document, *new_parent, DirtyFlags::LAYOUT_PAINT);
                }
                MutationKind::Attribute { node, name } => {
                    if matches!(name.as_str(), "id" | "class" | "style") {
                        set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                        let parent = document.node(*node).parent;
                        set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                    }
                }
                MutationKind::CharacterData { node } => {
                    set.mark(*node, DirtyFlags::LAYOUT_PAINT);
                    let parent = document.node(*node).parent;
                    set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                }
            }
        }

        set
    }
'''
new_invalidation = '''impl InvalidationSet {
    pub fn from_document_since(document: &Document, generation: u64) -> Self {
        let dependencies = SelectorInvalidationDependencies::default();
        Self::from_document_since_with_dependencies(document, generation, &dependencies)
    }

    pub fn from_document_since_with_styles(
        document: &Document,
        generation: u64,
        styles: &StyleSet,
    ) -> Self {
        let dependencies = styles.invalidation_dependencies();
        Self::from_document_since_with_dependencies(document, generation, &dependencies)
    }

    pub fn from_document_since_with_dependencies(
        document: &Document,
        generation: u64,
        dependencies: &SelectorInvalidationDependencies,
    ) -> Self {
        let mut set = Self {
            entries: BTreeMap::new(),
            through_generation: document.generation(),
        };

        for record in document.mutation_records_since(generation) {
            match &record.kind {
                MutationKind::NodeCreated { node } => {
                    set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                }
                MutationKind::ChildAdded { parent, child } => {
                    set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    set.mark_ancestors(document, Some(*parent), DirtyFlags::LAYOUT_PAINT);
                    set.mark_structural_dependents(document, *parent, *child, dependencies);
                }
                MutationKind::Reparented {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    set.mark(*child, DirtyFlags::STYLE_LAYOUT_PAINT);
                    set.mark_ancestors(document, *old_parent, DirtyFlags::LAYOUT_PAINT);
                    set.mark_ancestors(document, *new_parent, DirtyFlags::LAYOUT_PAINT);
                    if dependencies.has_scope(SelectorDependencyScope::Descendants) {
                        mark_subtree(
                            document,
                            *child,
                            &mut set,
                            DirtyFlags::STYLE_LAYOUT_PAINT,
                        );
                    }
                    if dependencies.has_scope(SelectorDependencyScope::FollowingSiblings) {
                        if let Some(old_parent) = old_parent {
                            set.mark_child_subtrees(
                                document,
                                *old_parent,
                                DirtyFlags::STYLE_LAYOUT_PAINT,
                            );
                        }
                        if let Some(new_parent) = new_parent {
                            set.mark_following_sibling_subtrees(
                                document,
                                *new_parent,
                                *child,
                                DirtyFlags::STYLE_LAYOUT_PAINT,
                            );
                        }
                    }
                }
                MutationKind::Attribute { node, name } => {
                    if matches!(name.as_str(), "id" | "class" | "style") {
                        set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                        let parent = document.node(*node).parent;
                        set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                    }
                    if matches!(name.as_str(), "id" | "class") {
                        set.mark_relational_dependents(document, *node, name, dependencies);
                    }
                }
                MutationKind::CharacterData { node } => {
                    set.mark(*node, DirtyFlags::LAYOUT_PAINT);
                    let parent = document.node(*node).parent;
                    set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                }
            }
        }

        set
    }
'''
text = replace_once(text, old_invalidation, new_invalidation, "invalidation implementation")

text = replace_once(
    text,
    '''    fn mark_ancestors(&mut self, document: &Document, mut node: Option<NodeId>, flags: DirtyFlags) {
        while let Some(current) = node {
            self.mark(current, flags);
            node = document.node(current).parent;
        }
    }
}
''',
    '''    fn mark_ancestors(&mut self, document: &Document, mut node: Option<NodeId>, flags: DirtyFlags) {
        while let Some(current) = node {
            self.mark(current, flags);
            node = document.node(current).parent;
        }
    }

    fn mark_relational_dependents(
        &mut self,
        document: &Document,
        node: NodeId,
        attribute: &str,
        dependencies: &SelectorInvalidationDependencies,
    ) {
        for dependency in dependencies.for_attribute(attribute) {
            match dependency.scope {
                SelectorDependencyScope::Descendants => {
                    for child in document.children(node) {
                        mark_subtree(document, *child, self, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                }
                SelectorDependencyScope::FollowingSiblings => {
                    if let Some(parent) = document.node(node).parent {
                        self.mark_following_sibling_subtrees(
                            document,
                            parent,
                            node,
                            DirtyFlags::STYLE_LAYOUT_PAINT,
                        );
                    }
                }
            }
        }
    }

    fn mark_structural_dependents(
        &mut self,
        document: &Document,
        parent: NodeId,
        child: NodeId,
        dependencies: &SelectorInvalidationDependencies,
    ) {
        if dependencies.has_scope(SelectorDependencyScope::Descendants) {
            mark_subtree(document, child, self, DirtyFlags::STYLE_LAYOUT_PAINT);
        }
        if dependencies.has_scope(SelectorDependencyScope::FollowingSiblings) {
            self.mark_following_sibling_subtrees(
                document,
                parent,
                child,
                DirtyFlags::STYLE_LAYOUT_PAINT,
            );
        }
    }

    fn mark_following_sibling_subtrees(
        &mut self,
        document: &Document,
        parent: NodeId,
        node: NodeId,
        flags: DirtyFlags,
    ) {
        let children = document.children(parent);
        let Some(position) = children.iter().position(|child| *child == node) else {
            self.mark_child_subtrees(document, parent, flags);
            return;
        };
        for sibling in children.iter().skip(position + 1) {
            mark_subtree(document, *sibling, self, flags);
        }
    }

    fn mark_child_subtrees(&mut self, document: &Document, parent: NodeId, flags: DirtyFlags) {
        for child in document.children(parent) {
            mark_subtree(document, *child, self, flags);
        }
    }
}
''',
    "invalidation helpers",
)

# Add regression tests before edge shorthand test.
test_anchor = '''    #[test]
    fn parses_css_edge_shorthand() {
'''
tests = '''    #[test]
    fn style_sharing_key_canonicalizes_class_order() {
        let mut document = Document::new();
        let mut first_attributes = BTreeMap::new();
        first_attributes.insert("class".into(), "card featured".into());
        first_attributes.insert("style".into(), "width:10px".into());
        let first = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html("div").with_attributes(first_attributes)),
            )
            .unwrap();

        let mut second_attributes = BTreeMap::new();
        second_attributes.insert("class".into(), "featured card card".into());
        second_attributes.insert("style".into(), "width:10px".into());
        let second = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html("div").with_attributes(second_attributes)),
            )
            .unwrap();

        assert_eq!(
            style_sharing_key(&document, first),
            style_sharing_key(&document, second)
        );
    }

    #[test]
    fn relational_dependencies_disable_local_style_sharing() {
        let mut stylesheet = Stylesheet::parse(
            StyleSource::author(1, "test"),
            ".target { width:10px; }",
        );
        stylesheet.rules[0].dependencies.push(SelectorDependency {
            trigger: SelectorInvalidationKey {
                tag: None,
                id: None,
                classes: BTreeSet::from(["theme".into()]),
            },
            scope: SelectorDependencyScope::Descendants,
        });
        let styles = StyleSet {
            stylesheets: vec![stylesheet],
        };

        assert!(!styles.local_style_sharing_safe());
        assert_eq!(styles.invalidation_dependencies().entries().len(), 1);
    }

    #[test]
    fn descendant_dependency_invalidates_subtree_when_trigger_class_is_removed() {
        let mut document = Document::new();
        let mut parent_attributes = BTreeMap::new();
        parent_attributes.insert("class".into(), "theme".into());
        let parent = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html("section").with_attributes(parent_attributes)),
            )
            .unwrap();
        let child = document
            .append_new(parent, NodeKind::Element(ElementData::html("div")))
            .unwrap();
        let grandchild = document
            .append_new(child, NodeKind::Element(ElementData::html("span")))
            .unwrap();
        let generation = document.generation();
        document.remove_attribute(parent, "class").unwrap();

        let mut dependencies = SelectorInvalidationDependencies::new();
        dependencies.register(SelectorDependency {
            trigger: SelectorInvalidationKey {
                tag: None,
                id: None,
                classes: BTreeSet::from(["theme".into()]),
            },
            scope: SelectorDependencyScope::Descendants,
        });
        let invalidation = InvalidationSet::from_document_since_with_dependencies(
            &document,
            generation,
            &dependencies,
        );

        assert_eq!(
            invalidation.entries.get(&child),
            Some(&DirtyFlags::STYLE_LAYOUT_PAINT)
        );
        assert_eq!(
            invalidation.entries.get(&grandchild),
            Some(&DirtyFlags::STYLE_LAYOUT_PAINT)
        );
    }

    #[test]
    fn sibling_dependency_invalidates_following_sibling_subtrees() {
        let mut document = Document::new();
        let mut trigger_attributes = BTreeMap::new();
        trigger_attributes.insert("id".into(), "lead".into());
        let trigger = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html("div").with_attributes(trigger_attributes)),
            )
            .unwrap();
        let sibling = document
            .append_new(document.root(), NodeKind::Element(ElementData::html("div")))
            .unwrap();
        let nested = document
            .append_new(sibling, NodeKind::Element(ElementData::html("span")))
            .unwrap();
        let trailing = document
            .append_new(document.root(), NodeKind::Element(ElementData::html("div")))
            .unwrap();
        let generation = document.generation();
        document.remove_attribute(trigger, "id").unwrap();

        let mut dependencies = SelectorInvalidationDependencies::new();
        dependencies.register(SelectorDependency {
            trigger: SelectorInvalidationKey {
                tag: None,
                id: Some("lead".into()),
                classes: BTreeSet::new(),
            },
            scope: SelectorDependencyScope::FollowingSiblings,
        });
        let invalidation = InvalidationSet::from_document_since_with_dependencies(
            &document,
            generation,
            &dependencies,
        );

        for node in [sibling, nested, trailing] {
            assert_eq!(
                invalidation.entries.get(&node),
                Some(&DirtyFlags::STYLE_LAYOUT_PAINT)
            );
        }
    }

'''
text = replace_once(text, test_anchor, tests + test_anchor, "style regression test anchor")
css.write_text(text)

engine = Path("crates/rarog-engine/src/lib.rs")
text = engine.read_text()
text = replace_once(
    text,
    '''    pub fn capture(&mut self, document: &Document) {
        let delta = InvalidationSet::from_document_since(document, self.through_generation);
''',
    '''    pub fn capture(&mut self, document: &Document, styles: &StyleSet) {
        let delta = InvalidationSet::from_document_since_with_styles(
            document,
            self.through_generation,
            styles,
        );
''',
    "dirty capture signature",
)
text = replace_once(
    text,
    '''        self.dirty.capture(&self.document);
''',
    '''        self.dirty.capture(&self.document, &self.styles);
''',
    "dirty capture call",
)
engine.write_text(text)

backlog = Path("docs/R0-BACKLOG.md")
text = backlog.read_text()
text = text.replace("- [ ] style sharing/cache design note", "- [x] style sharing/cache design note")
text = text.replace(
    "- [ ] descendant/sibling selector invalidation dependencies",
    "- [x] descendant/sibling selector invalidation dependencies",
)
backlog.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
text = architecture.read_text()
anchor = '''These flags are deliberately conservative. `rarog-engine` persists them in `DirtyState` across DOM generations until a render update consumes them.\n'''
addition = anchor + '''\n### Relational invalidation and style sharing\n\nR0 now has an explicit `SelectorInvalidationDependencies` boundary for selector relationships that can make a mutation affect nodes other than the mutated element. A dependency records the local trigger key plus a conservative scope: descendants or following siblings. The bootstrap CSS parser still accepts only simple selectors, so it produces no relational dependencies itself; a future standards parser can populate the same rule-level dependency metadata without changing the DOM mutation journal or engine dirty-state API.\n\nAttribute invalidation deliberately keys on the changed attribute category (`id` or `class`) rather than only the post-mutation value. This is necessary because the R0 mutation journal does not retain old attribute values: removing a trigger must invalidate the same dependent nodes as adding it. Structural insert/reparent operations conservatively invalidate affected descendant or sibling subtrees when the corresponding dependency scope exists.\n\n`StyleSharingKey` captures the local inputs that are sufficient for the current bootstrap selector/cascade model: namespace, tag, ID, canonicalized classes and inline style. Local style sharing is considered safe only while the active rule set has no relational dependencies. R0 does not install a process-global computed-style cache; any future cache must be bounded to a document/style-set lifetime and must expand or disable its key when inheritance, pseudo-state, relational selectors or other contextual inputs become observable. See ADR-0026.\n'''
text = replace_once(text, anchor, addition, "architecture invalidation anchor")
architecture.write_text(text)

Path("docs/adr/ADR-0026-style-sharing-and-relational-invalidation.md").write_text(
    '''# ADR-0026: Style sharing and relational invalidation\n\n## Status\n\nAccepted.\n\n## Context\n\nThe bootstrap style system originally invalidated only the element whose `id`, `class` or inline style changed. That is sufficient while selectors are local simple compounds, but descendant and sibling combinators make selector matching depend on relationships outside the subject element. A future style cache also needs an explicit statement of which inputs make two computed styles shareable.\n\n## Decision\n\nStyle rules may carry `SelectorDependency` metadata independently of their current bootstrap matching implementation. Each dependency identifies a trigger `SelectorInvalidationKey` and a conservative scope: `Descendants` or `FollowingSiblings`. `StyleSet` aggregates these dependencies and the engine feeds them into DOM-mutation invalidation.\n\nFor `id` and `class` mutations, invalidation is based on the changed attribute category rather than only the new value. This intentionally over-invalidates because the R0 mutation record does not retain the old attribute value; it therefore remains correct when a selector trigger is removed. Structural insertion and reparenting conservatively mark affected subtrees whenever relational dependency scopes are present.\n\nR0 defines `StyleSharingKey` from the inputs sufficient for its local selector model: namespace, local tag name, ID, canonicalized class set and inline style. `StyleSet::local_style_sharing_safe` returns false when relational dependency metadata is present. No process-global computed-style cache is introduced in R0. A future cache must be bounded to a document/style-set lifetime and include or account for every observable contextual input before sharing.\n\n## Consequences\n\n- The DOM mutation journal remains independent of CSS selector implementation details.\n- A standards-oriented selector parser can populate relational dependency metadata without changing engine invalidation ownership.\n- Trigger removal is conservatively correct despite the current mutation journal storing no old attribute value.\n- Sibling/descendant invalidation may over-invalidate in R0; correctness is preferred over premature precision.\n- Current simple selectors remain local and produce no relational dependencies automatically.\n- This ADR does not implement combinator parsing/matching, inheritance, pseudo-class state or a production computed-style cache.\n'''
)
