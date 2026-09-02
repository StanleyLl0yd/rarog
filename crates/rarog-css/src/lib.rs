mod selector;
mod syntax;

pub use selector::{
    AttributeSelector, Combinator, CompoundSelector, PseudoClass, Selector, parse_selector,
};

use rarog_dom::{Document, MutationKind, NodeId, NodeKind};
use rarog_types::Color;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeSizes {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn all(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }

    pub fn non_negative(self) -> Self {
        Self::new(
            self.top.max(0.0),
            self.right.max(0.0),
            self.bottom.max(0.0),
            self.left.max(0.0),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputedStyle {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub margin: EdgeSizes,
    pub border_width: EdgeSizes,
    pub padding: EdgeSizes,
    pub color: Color,
    pub background: Color,
    pub border_color: Color,
    pub display_none: bool,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            margin: EdgeSizes::ZERO,
            border_width: EdgeSizes::ZERO,
            padding: EdgeSizes::ZERO,
            color: Color::BLACK,
            background: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            display_none: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StyleSourceId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StyleOrigin {
    UserAgent,
    Author,
    Inline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CascadeLayer(pub u16);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleSource {
    pub id: StyleSourceId,
    pub origin: StyleOrigin,
    pub layer: CascadeLayer,
    pub label: String,
}

impl StyleSource {
    pub fn user_agent() -> Self {
        Self {
            id: StyleSourceId(0),
            origin: StyleOrigin::UserAgent,
            layer: CascadeLayer(0),
            label: "rarog-user-agent".into(),
        }
    }

    pub fn author(id: u32, label: impl Into<String>) -> Self {
        Self {
            id: StyleSourceId(id),
            origin: StyleOrigin::Author,
            layer: CascadeLayer(0),
            label: label.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Specificity {
    pub ids: u16,
    pub classes: u16,
    pub types: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectorInvalidationKey {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: BTreeSet<String>,
    pub attributes: BTreeSet<String>,
}

impl SelectorInvalidationKey {
    pub fn depends_on_attribute(&self, name: &str) -> bool {
        match name {
            "id" => self.id.is_some(),
            "class" => !self.classes.is_empty(),
            _ => self.attributes.contains(name),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectorDependencyScope {
    SelfNode,
    Descendants,
    FollowingSiblings,
    SiblingSet,
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
        self.entries
            .iter()
            .any(|dependency| dependency.scope == scope)
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
    let node = document.node(node)?;
    let NodeKind::Element(element) = &node.kind else {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PropertyId {
    Width,
    Height,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    BorderTopWidth,
    BorderRightWidth,
    BorderBottomWidth,
    BorderLeftWidth,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Color,
    BackgroundColor,
    BorderColor,
    Display,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayValue {
    Block,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CssWideKeyword {
    Initial,
    Inherit,
    Unset,
    Revert,
    RevertLayer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyValue {
    Length(f32),
    Color(Color),
    Display(DisplayValue),
    CssWide(CssWideKeyword),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Declaration {
    pub property: PropertyId,
    pub value: PropertyValue,
    pub important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleRule {
    pub selector: Selector,
    pub specificity: Specificity,
    pub declarations: Vec<Declaration>,
    pub source_order: u32,
    pub dependencies: Vec<SelectorDependency>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet {
    pub source: StyleSource,
    pub rules: Vec<StyleRule>,
}

impl Stylesheet {
    pub fn parse(source: StyleSource, input: &str) -> Self {
        let mut rules = Vec::new();
        let mut source_order = 0u32;

        for parsed_rule in syntax::parse_stylesheet(input) {
            let declarations = declarations_from_syntax(&parsed_rule.declarations);
            if declarations.is_empty() {
                continue;
            }
            for selector_text in parsed_rule.selectors {
                if let Some(selector) = parse_selector(&selector_text) {
                    let dependencies = selector.dependencies();
                    rules.push(StyleRule {
                        specificity: selector.specificity(),
                        selector,
                        declarations: declarations.clone(),
                        source_order,
                        dependencies,
                    });
                    source_order = source_order.saturating_add(1);
                }
            }
        }

        Self { source, rules }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleSet {
    pub stylesheets: Vec<Stylesheet>,
}

impl StyleSet {
    pub fn for_document(document: &Document) -> Self {
        let mut set = Self::default();
        set.stylesheets.push(Stylesheet::parse(
            StyleSource::user_agent(),
            "body { margin: 8px; background-color: white; } style { display: none; }",
        ));

        let mut author_sources = Vec::new();
        collect_style_elements(document, document.root(), &mut author_sources);
        for (index, css) in author_sources.into_iter().enumerate() {
            set.stylesheets.push(Stylesheet::parse(
                StyleSource::author(index as u32 + 1, format!("style-element-{}", index + 1)),
                &css,
            ));
        }
        set
    }

    pub fn rule_count(&self) -> usize {
        self.stylesheets
            .iter()
            .map(|stylesheet| stylesheet.rules.len())
            .fold(0usize, usize::saturating_add)
    }

    pub fn invalidation_dependencies(&self) -> SelectorInvalidationDependencies {
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
        for (sheet_index, sheet) in self.stylesheets.iter().enumerate() {
            output.push_str(&format!(
                "sheet={sheet_index}|origin={:?}|layer={}|label={}\n",
                sheet.source.origin, sheet.source.layer.0, sheet.source.label
            ));
            for rule in &sheet.rules {
                output.push_str(&format!(
                    " rule={}|selector={}|specificity={},{},{}|decls={}\n",
                    rule.source_order,
                    selector_snapshot(&rule.selector),
                    rule.specificity.ids,
                    rule.specificity.classes,
                    rule.specificity.types,
                    rule.declarations.len()
                ));
            }
        }
        output
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CascadePriority {
    important: bool,
    origin: StyleOrigin,
    layer: CascadeLayer,
    specificity: Specificity,
    sheet_order: u32,
    rule_order: u32,
    declaration_order: u32,
}

impl Ord for CascadePriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        cascade_origin_rank(self.important, self.origin)
            .cmp(&cascade_origin_rank(other.important, other.origin))
            .then_with(|| {
                cascade_layer_rank(self.important, self.layer)
                    .cmp(&cascade_layer_rank(other.important, other.layer))
            })
            .then_with(|| self.specificity.cmp(&other.specificity))
            .then_with(|| self.sheet_order.cmp(&other.sheet_order))
            .then_with(|| self.rule_order.cmp(&other.rule_order))
            .then_with(|| self.declaration_order.cmp(&other.declaration_order))
    }
}

impl PartialOrd for CascadePriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn cascade_origin_rank(important: bool, origin: StyleOrigin) -> u8 {
    match (important, origin) {
        (false, StyleOrigin::UserAgent) => 0,
        (false, StyleOrigin::Author | StyleOrigin::Inline) => 1,
        (true, StyleOrigin::Author | StyleOrigin::Inline) => 2,
        (true, StyleOrigin::UserAgent) => 3,
    }
}

fn cascade_layer_rank(important: bool, layer: CascadeLayer) -> u16 {
    if important {
        u16::MAX - layer.0
    } else {
        layer.0
    }
}

#[derive(Clone, Copy, Debug)]
struct CascadeCandidate {
    priority: CascadePriority,
    value: PropertyValue,
}

pub fn computed_style(document: &Document, node: NodeId, styles: &StyleSet) -> ComputedStyle {
    let mut lineage = Vec::new();
    let mut current = Some(node);
    while let Some(node) = current {
        let Some(dom_node) = document.node(node) else {
            return ComputedStyle::default();
        };
        lineage.push(node);
        current = dom_node.parent;
    }

    let mut parent_style = None;
    let mut style = ComputedStyle::default();
    for node in lineage.into_iter().rev() {
        style = computed_style_with_parent(document, node, styles, parent_style);
        parent_style = Some(style);
    }
    style
}

pub fn computed_style_with_parent(
    document: &Document,
    node: NodeId,
    styles: &StyleSet,
    parent_style: Option<ComputedStyle>,
) -> ComputedStyle {
    let Some(dom_node) = document.node(node) else {
        return ComputedStyle::default();
    };
    let NodeKind::Element(element) = &dom_node.kind else {
        return inherited_style(parent_style);
    };

    let mut candidates = BTreeMap::<PropertyId, Vec<CascadeCandidate>>::new();

    for (sheet_order, stylesheet) in styles.stylesheets.iter().enumerate() {
        for rule in &stylesheet.rules {
            if !rule.selector.matches(document, node) {
                continue;
            }
            apply_declarations(
                &mut candidates,
                &rule.declarations,
                CascadePriority {
                    important: false,
                    origin: stylesheet.source.origin,
                    layer: stylesheet.source.layer,
                    specificity: rule.specificity,
                    sheet_order: sheet_order as u32,
                    rule_order: rule.source_order,
                    declaration_order: 0,
                },
            );
        }
    }

    if let Some(inline) = element.attributes.get("style") {
        let declarations = parse_declarations(inline);
        apply_declarations(
            &mut candidates,
            &declarations,
            CascadePriority {
                important: false,
                origin: StyleOrigin::Inline,
                layer: CascadeLayer(0),
                specificity: Specificity {
                    ids: u16::MAX,
                    classes: u16::MAX,
                    types: u16::MAX,
                },
                sheet_order: u32::MAX,
                rule_order: u32::MAX,
                declaration_order: 0,
            },
        );
    }

    style_from_candidates(&candidates, parent_style)
}

fn inherited_style(parent_style: Option<ComputedStyle>) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    if let Some(parent) = parent_style {
        style.color = parent.color;
    }
    style
}

fn apply_declarations(
    candidates: &mut BTreeMap<PropertyId, Vec<CascadeCandidate>>,
    declarations: &[Declaration],
    base_priority: CascadePriority,
) {
    for (declaration_order, declaration) in declarations.iter().enumerate() {
        let priority = CascadePriority {
            important: declaration.important,
            declaration_order: declaration_order as u32,
            ..base_priority
        };
        candidates
            .entry(declaration.property)
            .or_default()
            .push(CascadeCandidate {
                priority,
                value: declaration.value,
            });
    }
}

fn style_from_candidates(
    candidates: &BTreeMap<PropertyId, Vec<CascadeCandidate>>,
    parent_style: Option<ComputedStyle>,
) -> ComputedStyle {
    let mut style = inherited_style(parent_style);
    for (property, property_candidates) in candidates {
        let Some(value) = resolve_cascade_value(property_candidates) else {
            continue;
        };
        match value {
            PropertyValue::CssWide(keyword) => {
                apply_css_wide(&mut style, *property, keyword, parent_style);
            }
            value => apply_property_value(&mut style, *property, value),
        }
    }
    style
}

fn resolve_cascade_value(candidates: &[CascadeCandidate]) -> Option<PropertyValue> {
    let mut ordered = candidates.to_vec();
    ordered.sort_by_key(|candidate| std::cmp::Reverse(candidate.priority));

    let mut reverted_author_origin = false;
    let mut reverted_layers = BTreeSet::new();

    for candidate in ordered {
        let origin = candidate.priority.origin;
        let author_origin = matches!(origin, StyleOrigin::Author | StyleOrigin::Inline);
        if (author_origin && reverted_author_origin)
            || reverted_layers.contains(&(origin, candidate.priority.layer))
        {
            continue;
        }

        match candidate.value {
            PropertyValue::CssWide(CssWideKeyword::Revert) => {
                if author_origin {
                    reverted_author_origin = true;
                } else {
                    return Some(PropertyValue::CssWide(CssWideKeyword::Unset));
                }
            }
            PropertyValue::CssWide(CssWideKeyword::RevertLayer) => {
                reverted_layers.insert((origin, candidate.priority.layer));
            }
            value => return Some(value),
        }
    }
    None
}

fn apply_css_wide(
    style: &mut ComputedStyle,
    property: PropertyId,
    keyword: CssWideKeyword,
    parent_style: Option<ComputedStyle>,
) {
    match keyword {
        CssWideKeyword::Initial => reset_property_to_initial(style, property),
        CssWideKeyword::Inherit => copy_property_from_parent(style, property, parent_style),
        CssWideKeyword::Unset if is_inherited_property(property) => {
            copy_property_from_parent(style, property, parent_style)
        }
        CssWideKeyword::Unset => reset_property_to_initial(style, property),
        CssWideKeyword::Revert | CssWideKeyword::RevertLayer => {}
    }
}

fn is_inherited_property(property: PropertyId) -> bool {
    property == PropertyId::Color
}

fn copy_property_from_parent(
    style: &mut ComputedStyle,
    property: PropertyId,
    parent_style: Option<ComputedStyle>,
) {
    let parent = parent_style.unwrap_or_default();
    match property {
        PropertyId::Width => style.width = parent.width,
        PropertyId::Height => style.height = parent.height,
        PropertyId::MarginTop => style.margin.top = parent.margin.top,
        PropertyId::MarginRight => style.margin.right = parent.margin.right,
        PropertyId::MarginBottom => style.margin.bottom = parent.margin.bottom,
        PropertyId::MarginLeft => style.margin.left = parent.margin.left,
        PropertyId::BorderTopWidth => style.border_width.top = parent.border_width.top,
        PropertyId::BorderRightWidth => style.border_width.right = parent.border_width.right,
        PropertyId::BorderBottomWidth => style.border_width.bottom = parent.border_width.bottom,
        PropertyId::BorderLeftWidth => style.border_width.left = parent.border_width.left,
        PropertyId::PaddingTop => style.padding.top = parent.padding.top,
        PropertyId::PaddingRight => style.padding.right = parent.padding.right,
        PropertyId::PaddingBottom => style.padding.bottom = parent.padding.bottom,
        PropertyId::PaddingLeft => style.padding.left = parent.padding.left,
        PropertyId::Color => style.color = parent.color,
        PropertyId::BackgroundColor => style.background = parent.background,
        PropertyId::BorderColor => style.border_color = parent.border_color,
        PropertyId::Display => style.display_none = parent.display_none,
    }
}

fn reset_property_to_initial(style: &mut ComputedStyle, property: PropertyId) {
    let initial = ComputedStyle::default();
    match property {
        PropertyId::Width => style.width = initial.width,
        PropertyId::Height => style.height = initial.height,
        PropertyId::MarginTop => style.margin.top = initial.margin.top,
        PropertyId::MarginRight => style.margin.right = initial.margin.right,
        PropertyId::MarginBottom => style.margin.bottom = initial.margin.bottom,
        PropertyId::MarginLeft => style.margin.left = initial.margin.left,
        PropertyId::BorderTopWidth => style.border_width.top = initial.border_width.top,
        PropertyId::BorderRightWidth => style.border_width.right = initial.border_width.right,
        PropertyId::BorderBottomWidth => style.border_width.bottom = initial.border_width.bottom,
        PropertyId::BorderLeftWidth => style.border_width.left = initial.border_width.left,
        PropertyId::PaddingTop => style.padding.top = initial.padding.top,
        PropertyId::PaddingRight => style.padding.right = initial.padding.right,
        PropertyId::PaddingBottom => style.padding.bottom = initial.padding.bottom,
        PropertyId::PaddingLeft => style.padding.left = initial.padding.left,
        PropertyId::Color => style.color = initial.color,
        PropertyId::BackgroundColor => style.background = initial.background,
        PropertyId::BorderColor => style.border_color = initial.border_color,
        PropertyId::Display => style.display_none = initial.display_none,
    }
}

fn apply_property_value(style: &mut ComputedStyle, property: PropertyId, value: PropertyValue) {
    match (property, value) {
        (PropertyId::Width, PropertyValue::Length(value)) => style.width = Some(value.max(0.0)),
        (PropertyId::Height, PropertyValue::Length(value)) => style.height = Some(value.max(0.0)),
        (PropertyId::MarginTop, PropertyValue::Length(value)) => style.margin.top = value,
        (PropertyId::MarginRight, PropertyValue::Length(value)) => style.margin.right = value,
        (PropertyId::MarginBottom, PropertyValue::Length(value)) => style.margin.bottom = value,
        (PropertyId::MarginLeft, PropertyValue::Length(value)) => style.margin.left = value,
        (PropertyId::BorderTopWidth, PropertyValue::Length(value)) => {
            style.border_width.top = value.max(0.0)
        }
        (PropertyId::BorderRightWidth, PropertyValue::Length(value)) => {
            style.border_width.right = value.max(0.0)
        }
        (PropertyId::BorderBottomWidth, PropertyValue::Length(value)) => {
            style.border_width.bottom = value.max(0.0)
        }
        (PropertyId::BorderLeftWidth, PropertyValue::Length(value)) => {
            style.border_width.left = value.max(0.0)
        }
        (PropertyId::PaddingTop, PropertyValue::Length(value)) => {
            style.padding.top = value.max(0.0)
        }
        (PropertyId::PaddingRight, PropertyValue::Length(value)) => {
            style.padding.right = value.max(0.0)
        }
        (PropertyId::PaddingBottom, PropertyValue::Length(value)) => {
            style.padding.bottom = value.max(0.0)
        }
        (PropertyId::PaddingLeft, PropertyValue::Length(value)) => {
            style.padding.left = value.max(0.0)
        }
        (PropertyId::Color, PropertyValue::Color(color)) => style.color = color,
        (PropertyId::BackgroundColor, PropertyValue::Color(color)) => style.background = color,
        (PropertyId::BorderColor, PropertyValue::Color(color)) => style.border_color = color,
        (PropertyId::Display, PropertyValue::Display(display)) => {
            style.display_none = display == DisplayValue::None
        }
        (_, PropertyValue::CssWide(_)) | (_, _) => {}
    }
}

fn collect_style_elements(document: &Document, node: NodeId, output: &mut Vec<String>) {
    let Some(current) = document.node(node) else {
        return;
    };
    if let NodeKind::Element(element) = &current.kind {
        if element.tag_name.as_str() == "style" {
            let mut text = String::new();
            collect_text(document, node, &mut text);
            if !text.trim().is_empty() {
                output.push(text);
            }
            return;
        }
    }

    for child in document.children(node).unwrap_or(&[]) {
        collect_style_elements(document, *child, output);
    }
}

fn collect_text(document: &Document, node: NodeId, output: &mut String) {
    for child in document.children(node).unwrap_or(&[]) {
        match document.node(*child).map(|node| &node.kind) {
            Some(NodeKind::Text(text)) => {
                if !output.is_empty() {
                    output.push(' ');
                }
                output.push_str(text);
            }
            _ => collect_text(document, *child, output),
        }
    }
}

fn parse_declarations(input: &str) -> Vec<Declaration> {
    let declarations = syntax::parse_declarations(input);
    declarations_from_syntax(&declarations)
}

fn declarations_from_syntax(input: &[syntax::ParsedDeclaration]) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for declaration in input {
        append_property(
            &mut declarations,
            declaration.name.as_str(),
            declaration.value.as_str(),
            declaration.important,
        );
    }
    declarations
}

fn append_property(output: &mut Vec<Declaration>, name: &str, value: &str, important: bool) {
    if let Some(keyword) = parse_css_wide(value) {
        push_css_wide(output, name, keyword, important);
        return;
    }

    match name {
        "width" => push_length(output, PropertyId::Width, value, false, important),
        "height" => push_length(output, PropertyId::Height, value, false, important),
        "margin" => push_edges(
            output,
            value,
            [
                PropertyId::MarginTop,
                PropertyId::MarginRight,
                PropertyId::MarginBottom,
                PropertyId::MarginLeft,
            ],
            true,
            important,
        ),
        "padding" => push_edges(
            output,
            value,
            [
                PropertyId::PaddingTop,
                PropertyId::PaddingRight,
                PropertyId::PaddingBottom,
                PropertyId::PaddingLeft,
            ],
            false,
            important,
        ),
        "border-width" => push_edges(
            output,
            value,
            [
                PropertyId::BorderTopWidth,
                PropertyId::BorderRightWidth,
                PropertyId::BorderBottomWidth,
                PropertyId::BorderLeftWidth,
            ],
            false,
            important,
        ),
        "margin-top" => push_length(output, PropertyId::MarginTop, value, true, important),
        "margin-right" => push_length(output, PropertyId::MarginRight, value, true, important),
        "margin-bottom" => push_length(output, PropertyId::MarginBottom, value, true, important),
        "margin-left" => push_length(output, PropertyId::MarginLeft, value, true, important),
        "padding-top" => push_length(output, PropertyId::PaddingTop, value, false, important),
        "padding-right" => push_length(output, PropertyId::PaddingRight, value, false, important),
        "padding-bottom" => push_length(output, PropertyId::PaddingBottom, value, false, important),
        "padding-left" => push_length(output, PropertyId::PaddingLeft, value, false, important),
        "border-top-width" => {
            push_length(output, PropertyId::BorderTopWidth, value, false, important)
        }
        "border-right-width" => push_length(
            output,
            PropertyId::BorderRightWidth,
            value,
            false,
            important,
        ),
        "border-bottom-width" => push_length(
            output,
            PropertyId::BorderBottomWidth,
            value,
            false,
            important,
        ),
        "border-left-width" => {
            push_length(output, PropertyId::BorderLeftWidth, value, false, important)
        }
        "color" => {
            if let Some(color) = parse_color(value) {
                output.push(Declaration {
                    property: PropertyId::Color,
                    value: PropertyValue::Color(color),
                    important,
                });
            }
        }
        "background" | "background-color" => {
            if let Some(color) = parse_color(value) {
                output.push(Declaration {
                    property: PropertyId::BackgroundColor,
                    value: PropertyValue::Color(color),
                    important,
                });
            }
        }
        "border-color" => {
            if let Some(color) = parse_color(value) {
                output.push(Declaration {
                    property: PropertyId::BorderColor,
                    value: PropertyValue::Color(color),
                    important,
                });
            }
        }
        "display" => {
            let display = match value.to_ascii_lowercase().as_str() {
                "none" => Some(DisplayValue::None),
                "block" => Some(DisplayValue::Block),
                _ => None,
            };
            if let Some(display) = display {
                output.push(Declaration {
                    property: PropertyId::Display,
                    value: PropertyValue::Display(display),
                    important,
                });
            }
        }
        _ => {}
    }
}

fn parse_css_wide(value: &str) -> Option<CssWideKeyword> {
    match value.trim().to_ascii_lowercase().as_str() {
        "initial" => Some(CssWideKeyword::Initial),
        "inherit" => Some(CssWideKeyword::Inherit),
        "unset" => Some(CssWideKeyword::Unset),
        "revert" => Some(CssWideKeyword::Revert),
        "revert-layer" => Some(CssWideKeyword::RevertLayer),
        _ => None,
    }
}

fn push_css_wide(
    output: &mut Vec<Declaration>,
    name: &str,
    keyword: CssWideKeyword,
    important: bool,
) {
    let properties: &[PropertyId] = match name {
        "width" => &[PropertyId::Width],
        "height" => &[PropertyId::Height],
        "margin" => &[
            PropertyId::MarginTop,
            PropertyId::MarginRight,
            PropertyId::MarginBottom,
            PropertyId::MarginLeft,
        ],
        "padding" => &[
            PropertyId::PaddingTop,
            PropertyId::PaddingRight,
            PropertyId::PaddingBottom,
            PropertyId::PaddingLeft,
        ],
        "border-width" => &[
            PropertyId::BorderTopWidth,
            PropertyId::BorderRightWidth,
            PropertyId::BorderBottomWidth,
            PropertyId::BorderLeftWidth,
        ],
        "margin-top" => &[PropertyId::MarginTop],
        "margin-right" => &[PropertyId::MarginRight],
        "margin-bottom" => &[PropertyId::MarginBottom],
        "margin-left" => &[PropertyId::MarginLeft],
        "padding-top" => &[PropertyId::PaddingTop],
        "padding-right" => &[PropertyId::PaddingRight],
        "padding-bottom" => &[PropertyId::PaddingBottom],
        "padding-left" => &[PropertyId::PaddingLeft],
        "border-top-width" => &[PropertyId::BorderTopWidth],
        "border-right-width" => &[PropertyId::BorderRightWidth],
        "border-bottom-width" => &[PropertyId::BorderBottomWidth],
        "border-left-width" => &[PropertyId::BorderLeftWidth],
        "color" => &[PropertyId::Color],
        "background" | "background-color" => &[PropertyId::BackgroundColor],
        "border-color" => &[PropertyId::BorderColor],
        "display" => &[PropertyId::Display],
        _ => return,
    };
    for property in properties {
        output.push(Declaration {
            property: *property,
            value: PropertyValue::CssWide(keyword),
            important,
        });
    }
}

fn push_length(
    output: &mut Vec<Declaration>,
    property: PropertyId,
    value: &str,
    negative: bool,
    important: bool,
) {
    if let Some(mut value) = parse_px(value) {
        if !negative {
            value = value.max(0.0);
        }
        output.push(Declaration {
            property,
            value: PropertyValue::Length(value),
            important,
        });
    }
}

fn push_edges(
    output: &mut Vec<Declaration>,
    value: &str,
    properties: [PropertyId; 4],
    negative: bool,
    important: bool,
) {
    let Some(edges) = parse_edge_sizes(value) else {
        return;
    };
    let values = [edges.top, edges.right, edges.bottom, edges.left];
    for (property, mut value) in properties.into_iter().zip(values) {
        if !negative {
            value = value.max(0.0);
        }
        output.push(Declaration {
            property,
            value: PropertyValue::Length(value),
            important,
        });
    }
}

fn parse_px(value: &str) -> Option<f32> {
    let parsed = value
        .trim()
        .strip_suffix("px")
        .unwrap_or(value.trim())
        .parse::<f32>()
        .ok()?;
    parsed.is_finite().then_some(parsed)
}

fn parse_edge_sizes(value: &str) -> Option<EdgeSizes> {
    let values = value
        .split_whitespace()
        .map(parse_px)
        .collect::<Option<Vec<_>>>()?;

    match values.as_slice() {
        [all] => Some(EdgeSizes::all(*all)),
        [vertical, horizontal] => Some(EdgeSizes::new(
            *vertical,
            *horizontal,
            *vertical,
            *horizontal,
        )),
        [top, horizontal, bottom] => Some(EdgeSizes::new(*top, *horizontal, *bottom, *horizontal)),
        [top, right, bottom, left] => Some(EdgeSizes::new(*top, *right, *bottom, *left)),
        _ => None,
    }
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim();
    match value.to_ascii_lowercase().as_str() {
        "transparent" => Some(Color::TRANSPARENT),
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        _ => {
            let hex = value.strip_prefix('#')?;
            if hex.len() != 6 {
                return None;
            }
            Some(Color::rgb(
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub style: bool,
    pub layout: bool,
    pub paint: bool,
}

impl DirtyFlags {
    pub const STYLE_LAYOUT_PAINT: Self = Self {
        style: true,
        layout: true,
        paint: true,
    };

    pub const LAYOUT_PAINT: Self = Self {
        style: false,
        layout: true,
        paint: true,
    };

    fn merge(&mut self, other: Self) {
        self.style |= other.style;
        self.layout |= other.layout;
        self.paint |= other.paint;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InvalidationSet {
    pub entries: BTreeMap<NodeId, DirtyFlags>,
    pub through_generation: u64,
}

impl InvalidationSet {
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

        let records = match document.mutation_records_since(generation) {
            Ok(records) => records,
            Err(_) => return Self::for_stylesheet_change(document),
        };

        for record in records {
            match &record.kind {
                MutationKind::NodeCreated { node } => {
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
                }
                MutationKind::Reparented {
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
                    if child_connected
                        && dependencies.has_scope(SelectorDependencyScope::Descendants)
                    {
                        mark_subtree(document, *child, &mut set, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                    if (old_connected || new_connected)
                        && dependencies.has_scope(SelectorDependencyScope::FollowingSiblings)
                    {
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
                    if (old_connected || new_connected)
                        && dependencies.has_scope(SelectorDependencyScope::SiblingSet)
                    {
                        if let Some(old_parent) = old_parent {
                            set.mark_child_subtrees(
                                document,
                                *old_parent,
                                DirtyFlags::STYLE_LAYOUT_PAINT,
                            );
                        }
                        if let Some(new_parent) = new_parent {
                            set.mark_child_subtrees(
                                document,
                                *new_parent,
                                DirtyFlags::STYLE_LAYOUT_PAINT,
                            );
                        }
                    }
                }
                MutationKind::Attribute { node, name } => {
                    if !document.is_connected(*node) {
                        continue;
                    }
                    let mut affects_layout = false;
                    if matches!(name.as_str(), "id" | "class" | "style") {
                        set.mark(*node, DirtyFlags::STYLE_LAYOUT_PAINT);
                        affects_layout = true;
                    }
                    affects_layout |=
                        set.mark_selector_dependents(document, *node, name, dependencies);
                    if affects_layout {
                        let parent = document.node(*node).and_then(|node| node.parent);
                        set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                    }
                }
                MutationKind::CharacterData { node } => {
                    if !document.is_connected(*node) {
                        continue;
                    }
                    set.mark(*node, DirtyFlags::LAYOUT_PAINT);
                    let parent = document.node(*node).and_then(|node| node.parent);
                    set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                }
            }
        }

        set
    }

    pub fn for_stylesheet_change(document: &Document) -> Self {
        let mut set = Self {
            entries: BTreeMap::new(),
            through_generation: document.generation(),
        };
        mark_subtree(
            document,
            document.root(),
            &mut set,
            DirtyFlags::STYLE_LAYOUT_PAINT,
        );
        set
    }

    fn mark(&mut self, node: NodeId, flags: DirtyFlags) {
        self.entries.entry(node).or_default().merge(flags);
    }

    fn mark_ancestors(&mut self, document: &Document, mut node: Option<NodeId>, flags: DirtyFlags) {
        while let Some(current) = node {
            self.mark(current, flags);
            node = document.node(current).and_then(|node| node.parent);
        }
    }

    fn mark_selector_dependents(
        &mut self,
        document: &Document,
        node: NodeId,
        attribute: &str,
        dependencies: &SelectorInvalidationDependencies,
    ) -> bool {
        let mut affected = false;
        for dependency in dependencies.for_attribute(attribute) {
            affected = true;
            match dependency.scope {
                SelectorDependencyScope::SelfNode => {
                    self.mark(node, DirtyFlags::STYLE_LAYOUT_PAINT);
                }
                SelectorDependencyScope::Descendants => {
                    for child in document.children(node).unwrap_or(&[]) {
                        mark_subtree(document, *child, self, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                }
                SelectorDependencyScope::FollowingSiblings => {
                    if let Some(parent) = document.node(node).and_then(|node| node.parent) {
                        self.mark_following_sibling_subtrees(
                            document,
                            parent,
                            node,
                            DirtyFlags::STYLE_LAYOUT_PAINT,
                        );
                    }
                }
                SelectorDependencyScope::SiblingSet => {
                    if let Some(parent) = document.node(node).and_then(|node| node.parent) {
                        self.mark_child_subtrees(document, parent, DirtyFlags::STYLE_LAYOUT_PAINT);
                    }
                }
            }
        }
        affected
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
        if dependencies.has_scope(SelectorDependencyScope::SiblingSet) {
            self.mark_child_subtrees(document, parent, DirtyFlags::STYLE_LAYOUT_PAINT);
        }
    }

    fn mark_following_sibling_subtrees(
        &mut self,
        document: &Document,
        parent: NodeId,
        node: NodeId,
        flags: DirtyFlags,
    ) {
        let Some(children) = document.children(parent) else {
            return;
        };
        let Some(position) = children.iter().position(|child| *child == node) else {
            self.mark_child_subtrees(document, parent, flags);
            return;
        };
        for sibling in children.iter().skip(position + 1) {
            mark_subtree(document, *sibling, self, flags);
        }
    }

    fn mark_child_subtrees(&mut self, document: &Document, parent: NodeId, flags: DirtyFlags) {
        for child in document.children(parent).unwrap_or(&[]) {
            mark_subtree(document, *child, self, flags);
        }
    }
}

fn mark_subtree(document: &Document, node: NodeId, set: &mut InvalidationSet, flags: DirtyFlags) {
    set.mark(node, flags);
    for child in document.children(node).unwrap_or(&[]) {
        mark_subtree(document, *child, set, flags);
    }
}

fn selector_snapshot(selector: &Selector) -> String {
    selector.snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_dom::{ElementData, NodeKind};

    fn document_with_element(tag: &str, attributes: &[(&str, &str)]) -> (Document, NodeId) {
        let mut document = Document::new();
        let mut attrs = BTreeMap::new();
        for (name, value) in attributes {
            attrs.insert((*name).into(), (*value).into());
        }
        let node = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html(tag).with_attributes(attrs)),
            )
            .unwrap();
        (document, node)
    }

    #[test]
    fn selector_representation_tracks_specificity_and_matching() {
        let selector = parse_selector("div.card#hero").unwrap();
        let (document, node) =
            document_with_element("div", &[("id", "hero"), ("class", "card featured")]);

        assert_eq!(
            selector.specificity(),
            Specificity {
                ids: 1,
                classes: 1,
                types: 1,
            }
        );
        assert!(selector.matches(&document, node));
        assert_eq!(selector.invalidation_key().id.as_deref(), Some("hero"));
    }

    #[test]
    fn cascade_prefers_specific_rule_then_inline_style() {
        let (document, node) = document_with_element(
            "div",
            &[
                ("id", "hero"),
                ("class", "card"),
                ("style", "background:#000000"),
            ],
        );
        let stylesheet = Stylesheet::parse(
            StyleSource::author(1, "test"),
            ".card { background:#112233; } #hero { background:#445566; }",
        );
        let styles = StyleSet {
            stylesheets: vec![stylesheet],
        };

        let style = computed_style(&document, node, &styles);
        assert_eq!(style.background, Color::BLACK);
    }

    #[test]
    fn later_rule_wins_when_specificity_is_equal() {
        let (document, node) = document_with_element("div", &[("class", "card")]);
        let stylesheet = Stylesheet::parse(
            StyleSource::author(1, "test"),
            ".card { width:10px; } .card { width:20px; }",
        );
        let styles = StyleSet {
            stylesheets: vec![stylesheet],
        };

        assert_eq!(computed_style(&document, node, &styles).width, Some(20.0));
    }

    #[test]
    fn style_element_becomes_an_author_stylesheet() {
        let document = rarog_dom_document_from_parts();
        let styles = StyleSet::for_document(&document.0);
        let style = computed_style(&document.0, document.1, &styles);

        assert_eq!(style.width, Some(42.0));
    }

    #[test]
    fn dom_mutations_produce_granular_dirty_flags() {
        let (mut document, node) = document_with_element("div", &[]);
        let generation = document.generation();
        document.set_attribute(node, "class", "card").unwrap();

        let invalidation = InvalidationSet::from_document_since(&document, generation);
        assert_eq!(
            invalidation.entries.get(&node),
            Some(&DirtyFlags::STYLE_LAYOUT_PAINT)
        );
        assert_eq!(invalidation.through_generation, document.generation());
    }

    #[test]
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
        let mut stylesheet =
            Stylesheet::parse(StyleSource::author(1, "test"), ".target { width:10px; }");
        stylesheet.rules[0].dependencies.push(SelectorDependency {
            trigger: SelectorInvalidationKey {
                tag: None,
                id: None,
                classes: BTreeSet::from(["theme".into()]),
                attributes: BTreeSet::new(),
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
                attributes: BTreeSet::new(),
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
                attributes: BTreeSet::new(),
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

    #[test]
    fn parses_css_edge_shorthand() {
        assert_eq!(
            parse_edge_sizes("1px 2px 3px 4px"),
            Some(EdgeSizes::new(1.0, 2.0, 3.0, 4.0))
        );
        assert_eq!(
            parse_edge_sizes("8px 16px"),
            Some(EdgeSizes::new(8.0, 16.0, 8.0, 16.0))
        );
    }

    fn rarog_dom_document_from_parts() -> (Document, NodeId) {
        let mut document = Document::new();
        let style = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html("style")),
            )
            .unwrap();
        document
            .append_new(style, NodeKind::Text(".card { width:42px; }".into()))
            .unwrap();
        let mut attributes = BTreeMap::new();
        attributes.insert("class".into(), "card".into());
        let target = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData::html("div").with_attributes(attributes)),
            )
            .unwrap();
        (document, target)
    }
}

#[cfg(test)]
mod finite_geometry_tests {
    use super::*;

    #[test]
    fn detached_mutations_do_not_dirty_the_connected_document() {
        let mut document = Document::new();
        let detached = document
            .create_node(NodeKind::Element(rarog_dom::ElementData::html("div")))
            .unwrap();
        let generation = document.generation();
        document.set_attribute(detached, "class", "card").unwrap();

        let invalidation = InvalidationSet::from_document_since(&document, generation);
        assert!(invalidation.entries.is_empty());

        document.append_child(document.root(), detached).unwrap();
        let invalidation = InvalidationSet::from_document_since(&document, generation);
        assert!(invalidation.entries.contains_key(&detached));
    }

    #[test]
    fn non_finite_lengths_are_rejected() {
        assert_eq!(parse_px("NaNpx"), None);
        assert_eq!(parse_px("infpx"), None);
        assert_eq!(parse_px("-infpx"), None);
        assert_eq!(parse_px("12px"), Some(12.0));
    }
}
