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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlign {
    Baseline,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignItems {
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputedStyle {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub margin: EdgeSizes,
    pub border_width: EdgeSizes,
    pub padding: EdgeSizes,
    pub color: Color,
    pub background: Color,
    pub border_color: Color,
    pub display_none: bool,
    pub display_inline: bool,
    pub display_flex: bool,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub row_gap: f32,
    pub column_gap: f32,
    pub establishes_bfc: bool,
    pub vertical_align: VerticalAlign,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            margin: EdgeSizes::ZERO,
            border_width: EdgeSizes::ZERO,
            padding: EdgeSizes::ZERO,
            color: Color::BLACK,
            background: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            display_none: false,
            display_inline: false,
            display_flex: false,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            row_gap: 0.0,
            column_gap: 0.0,
            establishes_bfc: false,
            vertical_align: VerticalAlign::Baseline,
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
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
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
    FlexGrow,
    FlexShrink,
    JustifyContent,
    AlignItems,
    RowGap,
    ColumnGap,
    VerticalAlign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayValue {
    Block,
    Inline,
    Flex,
    FlowRoot,
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
    Auto,
    NoneKeyword,
    Color(Color),
    Display(DisplayValue),
    Number(f32),
    JustifyContent(JustifyContent),
    AlignItems(AlignItems),
    VerticalAlign(VerticalAlign),
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

    style_from_candidates(candidates, parent_style)
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
    candidates: BTreeMap<PropertyId, Vec<CascadeCandidate>>,
    parent_style: Option<ComputedStyle>,
) -> ComputedStyle {
    let mut style = inherited_style(parent_style);
    for (property, mut property_candidates) in candidates {
        let Some(value) = resolve_cascade_value(&mut property_candidates) else {
            continue;
        };
        match value {
            PropertyValue::CssWide(keyword) => {
                apply_css_wide(&mut style, property, keyword, parent_style);
            }
            value => apply_property_value(&mut style, property, value),
        }
    }
    style
}

fn resolve_cascade_value(candidates: &mut [CascadeCandidate]) -> Option<PropertyValue> {
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.priority));

    let mut reverted_author_origin = false;
    let mut reverted_layers = BTreeSet::new();

    for candidate in candidates.iter().copied() {
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
    copy_property_from_style(style, property, parent_style.unwrap_or_default());
}

fn reset_property_to_initial(style: &mut ComputedStyle, property: PropertyId) {
    copy_property_from_style(style, property, ComputedStyle::default());
}

fn copy_property_from_style(
    style: &mut ComputedStyle,
    property: PropertyId,
    source: ComputedStyle,
) {
    match property {
        PropertyId::Width => style.width = source.width,
        PropertyId::Height => style.height = source.height,
        PropertyId::MinWidth => style.min_width = source.min_width,
        PropertyId::MaxWidth => style.max_width = source.max_width,
        PropertyId::MinHeight => style.min_height = source.min_height,
        PropertyId::MaxHeight => style.max_height = source.max_height,
        PropertyId::MarginTop => style.margin.top = source.margin.top,
        PropertyId::MarginRight => style.margin.right = source.margin.right,
        PropertyId::MarginBottom => style.margin.bottom = source.margin.bottom,
        PropertyId::MarginLeft => style.margin.left = source.margin.left,
        PropertyId::BorderTopWidth => style.border_width.top = source.border_width.top,
        PropertyId::BorderRightWidth => style.border_width.right = source.border_width.right,
        PropertyId::BorderBottomWidth => style.border_width.bottom = source.border_width.bottom,
        PropertyId::BorderLeftWidth => style.border_width.left = source.border_width.left,
        PropertyId::PaddingTop => style.padding.top = source.padding.top,
        PropertyId::PaddingRight => style.padding.right = source.padding.right,
        PropertyId::PaddingBottom => style.padding.bottom = source.padding.bottom,
        PropertyId::PaddingLeft => style.padding.left = source.padding.left,
        PropertyId::Color => style.color = source.color,
        PropertyId::BackgroundColor => style.background = source.background,
        PropertyId::BorderColor => style.border_color = source.border_color,
        PropertyId::Display => {
            style.display_none = source.display_none;
            style.display_inline = source.display_inline;
            style.display_flex = source.display_flex;
            style.establishes_bfc = source.establishes_bfc;
        }
        PropertyId::FlexGrow => style.flex_grow = source.flex_grow,
        PropertyId::FlexShrink => style.flex_shrink = source.flex_shrink,
        PropertyId::JustifyContent => style.justify_content = source.justify_content,
        PropertyId::AlignItems => style.align_items = source.align_items,
        PropertyId::RowGap => style.row_gap = source.row_gap,
        PropertyId::ColumnGap => style.column_gap = source.column_gap,
        PropertyId::VerticalAlign => style.vertical_align = source.vertical_align,
    }
}

fn apply_property_value(style: &mut ComputedStyle, property: PropertyId, value: PropertyValue) {
    match (property, value) {
        (PropertyId::Width, PropertyValue::Length(value)) => style.width = Some(value.max(0.0)),
        (PropertyId::Width, PropertyValue::Auto) => style.width = None,
        (PropertyId::Height, PropertyValue::Length(value)) => style.height = Some(value.max(0.0)),
        (PropertyId::Height, PropertyValue::Auto) => style.height = None,
        (PropertyId::MinWidth, PropertyValue::Length(value)) => {
            style.min_width = Some(value.max(0.0))
        }
        (PropertyId::MinWidth, PropertyValue::Auto) => style.min_width = None,
        (PropertyId::MaxWidth, PropertyValue::Length(value)) => {
            style.max_width = Some(value.max(0.0))
        }
        (PropertyId::MaxWidth, PropertyValue::NoneKeyword) => style.max_width = None,
        (PropertyId::MinHeight, PropertyValue::Length(value)) => {
            style.min_height = Some(value.max(0.0))
        }
        (PropertyId::MinHeight, PropertyValue::Auto) => style.min_height = None,
        (PropertyId::MaxHeight, PropertyValue::Length(value)) => {
            style.max_height = Some(value.max(0.0))
        }
        (PropertyId::MaxHeight, PropertyValue::NoneKeyword) => style.max_height = None,
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
            style.display_none = display == DisplayValue::None;
            style.display_inline = display == DisplayValue::Inline;
            style.display_flex = display == DisplayValue::Flex;
            style.establishes_bfc = display == DisplayValue::FlowRoot;
        }
        (PropertyId::FlexGrow, PropertyValue::Number(value)) => style.flex_grow = value,
        (PropertyId::FlexShrink, PropertyValue::Number(value)) => style.flex_shrink = value,
        (PropertyId::JustifyContent, PropertyValue::JustifyContent(value)) => {
            style.justify_content = value
        }
        (PropertyId::AlignItems, PropertyValue::AlignItems(value)) => style.align_items = value,
        (PropertyId::RowGap, PropertyValue::Length(value)) => style.row_gap = value,
        (PropertyId::ColumnGap, PropertyValue::Length(value)) => style.column_gap = value,
        (PropertyId::VerticalAlign, PropertyValue::VerticalAlign(value)) => {
            style.vertical_align = value
        }
        (_, PropertyValue::CssWide(_)) | (_, _) => {}
    }
}

fn collect_style_elements(document: &Document, node: NodeId, output: &mut Vec<String>) {
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        let Some(current) = document.node(node) else {
            continue;
        };
        if matches!(&current.kind, NodeKind::Element(element) if element.tag_name.as_str() == "style")
        {
            let mut text = String::new();
            collect_text(document, node, &mut text);
            if !text.trim().is_empty() {
                output.push(text);
            }
            continue;
        }
        stack.extend(current.children.iter().rev().copied());
    }
}

fn collect_text(document: &Document, node: NodeId, output: &mut String) {
    let mut stack = document
        .children(node)
        .unwrap_or(&[])
        .iter()
        .rev()
        .copied()
        .collect::<Vec<_>>();
    while let Some(node) = stack.pop() {
        let Some(current) = document.node(node) else {
            continue;
        };
        match &current.kind {
            NodeKind::Text(text) => {
                if !output.is_empty() {
                    output.push(' ');
                }
                output.push_str(text);
            }
            NodeKind::Document | NodeKind::Element(_) => {
                stack.extend(current.children.iter().rev().copied());
            }
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
        "width" => push_sizing_value(output, PropertyId::Width, value, true, false, important),
        "height" => push_sizing_value(output, PropertyId::Height, value, true, false, important),
        "min-width" => {
            push_sizing_value(output, PropertyId::MinWidth, value, true, false, important)
        }
        "max-width" => {
            push_sizing_value(output, PropertyId::MaxWidth, value, false, true, important)
        }
        "min-height" => {
            push_sizing_value(output, PropertyId::MinHeight, value, true, false, important)
        }
        "max-height" => {
            push_sizing_value(output, PropertyId::MaxHeight, value, false, true, important)
        }
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
        "flex-grow" => push_non_negative_number(output, PropertyId::FlexGrow, value, important),
        "flex-shrink" => push_non_negative_number(output, PropertyId::FlexShrink, value, important),
        "row-gap" => push_gap_value(output, PropertyId::RowGap, value, important),
        "column-gap" => push_gap_value(output, PropertyId::ColumnGap, value, important),
        "gap" => push_gap_shorthand(output, value, important),
        "align-items" => {
            let value = value.trim();
            let value =
                if value.eq_ignore_ascii_case("stretch") || value.eq_ignore_ascii_case("normal") {
                    Some(AlignItems::Stretch)
                } else if value.eq_ignore_ascii_case("flex-start") {
                    Some(AlignItems::FlexStart)
                } else if value.eq_ignore_ascii_case("flex-end") {
                    Some(AlignItems::FlexEnd)
                } else if value.eq_ignore_ascii_case("center") {
                    Some(AlignItems::Center)
                } else {
                    None
                };
            if let Some(value) = value {
                output.push(Declaration {
                    property: PropertyId::AlignItems,
                    value: PropertyValue::AlignItems(value),
                    important,
                });
            }
        }
        "justify-content" => {
            let value = value.trim();
            let value = if value.eq_ignore_ascii_case("flex-start")
                || value.eq_ignore_ascii_case("normal")
            {
                Some(JustifyContent::FlexStart)
            } else if value.eq_ignore_ascii_case("flex-end") {
                Some(JustifyContent::FlexEnd)
            } else if value.eq_ignore_ascii_case("center") {
                Some(JustifyContent::Center)
            } else if value.eq_ignore_ascii_case("space-between") {
                Some(JustifyContent::SpaceBetween)
            } else if value.eq_ignore_ascii_case("space-around") {
                Some(JustifyContent::SpaceAround)
            } else if value.eq_ignore_ascii_case("space-evenly") {
                Some(JustifyContent::SpaceEvenly)
            } else {
                None
            };
            if let Some(value) = value {
                output.push(Declaration {
                    property: PropertyId::JustifyContent,
                    value: PropertyValue::JustifyContent(value),
                    important,
                });
            }
        }
        "display" => {
            let display = if value.eq_ignore_ascii_case("none") {
                Some(DisplayValue::None)
            } else if value.eq_ignore_ascii_case("block") {
                Some(DisplayValue::Block)
            } else if value.eq_ignore_ascii_case("inline") {
                Some(DisplayValue::Inline)
            } else if value.eq_ignore_ascii_case("flex") {
                Some(DisplayValue::Flex)
            } else if value.eq_ignore_ascii_case("flow-root") {
                Some(DisplayValue::FlowRoot)
            } else {
                None
            };
            if let Some(display) = display {
                output.push(Declaration {
                    property: PropertyId::Display,
                    value: PropertyValue::Display(display),
                    important,
                });
            }
        }
        "vertical-align" => {
            let value = value.trim();
            let value = if value.eq_ignore_ascii_case("baseline") {
                Some(VerticalAlign::Baseline)
            } else if value.eq_ignore_ascii_case("top") {
                Some(VerticalAlign::Top)
            } else if value.eq_ignore_ascii_case("bottom") {
                Some(VerticalAlign::Bottom)
            } else {
                None
            };
            if let Some(value) = value {
                output.push(Declaration {
                    property: PropertyId::VerticalAlign,
                    value: PropertyValue::VerticalAlign(value),
                    important,
                });
            }
        }
        _ => {}
    }
}

fn parse_css_wide(value: &str) -> Option<CssWideKeyword> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("initial") {
        Some(CssWideKeyword::Initial)
    } else if value.eq_ignore_ascii_case("inherit") {
        Some(CssWideKeyword::Inherit)
    } else if value.eq_ignore_ascii_case("unset") {
        Some(CssWideKeyword::Unset)
    } else if value.eq_ignore_ascii_case("revert") {
        Some(CssWideKeyword::Revert)
    } else if value.eq_ignore_ascii_case("revert-layer") {
        Some(CssWideKeyword::RevertLayer)
    } else {
        None
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
        "min-width" => &[PropertyId::MinWidth],
        "max-width" => &[PropertyId::MaxWidth],
        "min-height" => &[PropertyId::MinHeight],
        "max-height" => &[PropertyId::MaxHeight],
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
        "flex-grow" => &[PropertyId::FlexGrow],
        "flex-shrink" => &[PropertyId::FlexShrink],
        "justify-content" => &[PropertyId::JustifyContent],
        "align-items" => &[PropertyId::AlignItems],
        "row-gap" => &[PropertyId::RowGap],
        "column-gap" => &[PropertyId::ColumnGap],
        "gap" => &[PropertyId::RowGap, PropertyId::ColumnGap],
        "vertical-align" => &[PropertyId::VerticalAlign],
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

fn push_sizing_value(
    output: &mut Vec<Declaration>,
    property: PropertyId,
    value: &str,
    allow_auto: bool,
    allow_none: bool,
    important: bool,
) {
    let value = value.trim();
    if allow_auto && value.eq_ignore_ascii_case("auto") {
        output.push(Declaration {
            property,
            value: PropertyValue::Auto,
            important,
        });
        return;
    }
    if allow_none && value.eq_ignore_ascii_case("none") {
        output.push(Declaration {
            property,
            value: PropertyValue::NoneKeyword,
            important,
        });
        return;
    }
    push_length(output, property, value, false, important);
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

fn push_gap_shorthand(output: &mut Vec<Declaration>, value: &str, important: bool) {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    let (row, column) = match parts.as_slice() {
        [both] => {
            let Some(value) = parse_gap_component(both) else {
                return;
            };
            (value, value)
        }
        [row, column] => {
            let (Some(row), Some(column)) = (parse_gap_component(row), parse_gap_component(column))
            else {
                return;
            };
            (row, column)
        }
        _ => return,
    };

    output.push(Declaration {
        property: PropertyId::RowGap,
        value: PropertyValue::Length(row),
        important,
    });
    output.push(Declaration {
        property: PropertyId::ColumnGap,
        value: PropertyValue::Length(column),
        important,
    });
}

fn push_gap_value(
    output: &mut Vec<Declaration>,
    property: PropertyId,
    value: &str,
    important: bool,
) {
    let Some(value) = parse_gap_component(value) else {
        return;
    };
    output.push(Declaration {
        property,
        value: PropertyValue::Length(value),
        important,
    });
}

fn parse_gap_component(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("normal") {
        return Some(0.0);
    }
    let value = parse_px(value)?;
    (value >= 0.0).then_some(value)
}

fn push_non_negative_number(
    output: &mut Vec<Declaration>,
    property: PropertyId,
    value: &str,
    important: bool,
) {
    let Ok(value) = value.trim().parse::<f32>() else {
        return;
    };
    if !value.is_finite() || value < 0.0 {
        return;
    }
    output.push(Declaration {
        property,
        value: PropertyValue::Number(value),
        important,
    });
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
    if value.eq_ignore_ascii_case("transparent") {
        return Some(Color::TRANSPARENT);
    }
    if value.eq_ignore_ascii_case("white") {
        return Some(Color::WHITE);
    }
    if value.eq_ignore_ascii_case("black") {
        return Some(Color::BLACK);
    }

    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    Some(Color::rgb(
        u8::from_str_radix(hex.get(0..2)?, 16).ok()?,
        u8::from_str_radix(hex.get(2..4)?, 16).ok()?,
        u8::from_str_radix(hex.get(4..6)?, 16).ok()?,
    ))
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
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        set.mark(node, flags);
        if let Some(children) = document.children(node) {
            stack.extend(children.iter().rev().copied());
        }
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
    fn malformed_non_ascii_hex_color_is_rejected_without_panicking() {
        assert_eq!(parse_color("#aéabc"), None);
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
    fn ascii_keyword_values_remain_case_insensitive() {
        let declarations = parse_declarations(
            "display:FlOw-RoOt;vertical-align:TOP;color:BlAcK;width:AuTo;max-width:NoNe;margin-top:InItIaL",
        );

        assert!(declarations.contains(&Declaration {
            property: PropertyId::Display,
            value: PropertyValue::Display(DisplayValue::FlowRoot),
            important: false,
        }));
        assert!(declarations.contains(&Declaration {
            property: PropertyId::VerticalAlign,
            value: PropertyValue::VerticalAlign(VerticalAlign::Top),
            important: false,
        }));
        assert!(declarations.contains(&Declaration {
            property: PropertyId::Color,
            value: PropertyValue::Color(Color::BLACK),
            important: false,
        }));
        assert!(declarations.contains(&Declaration {
            property: PropertyId::Width,
            value: PropertyValue::Auto,
            important: false,
        }));
        assert!(declarations.contains(&Declaration {
            property: PropertyId::MaxWidth,
            value: PropertyValue::NoneKeyword,
            important: false,
        }));
        assert!(declarations.contains(&Declaration {
            property: PropertyId::MarginTop,
            value: PropertyValue::CssWide(CssWideKeyword::Initial),
            important: false,
        }));
    }

    #[test]
    fn align_items_parses_bounded_cross_axis_values() {
        let declarations = parse_declarations(
            "align-items:flex-start;align-items:flex-end;align-items:center;align-items:stretch",
        );
        for value in [
            AlignItems::FlexStart,
            AlignItems::FlexEnd,
            AlignItems::Center,
            AlignItems::Stretch,
        ] {
            assert!(declarations.contains(&Declaration {
                property: PropertyId::AlignItems,
                value: PropertyValue::AlignItems(value),
                important: false,
            }));
        }

        let normal = parse_declarations("align-items:NoRmAl");
        assert_eq!(
            normal,
            vec![Declaration {
                property: PropertyId::AlignItems,
                value: PropertyValue::AlignItems(AlignItems::Stretch),
                important: false,
            }]
        );
        assert!(parse_declarations("align-items:baseline").is_empty());

        let mut style = ComputedStyle::default();
        assert_eq!(style.align_items, AlignItems::Stretch);
        apply_property_value(
            &mut style,
            PropertyId::AlignItems,
            PropertyValue::AlignItems(AlignItems::Center),
        );
        assert_eq!(style.align_items, AlignItems::Center);
    }

    #[test]
    fn gap_shorthand_and_longhands_parse_to_non_negative_used_lengths() {
        let declarations = parse_declarations("gap:6px 10px");
        assert_eq!(
            declarations,
            vec![
                Declaration {
                    property: PropertyId::RowGap,
                    value: PropertyValue::Length(6.0),
                    important: false,
                },
                Declaration {
                    property: PropertyId::ColumnGap,
                    value: PropertyValue::Length(10.0),
                    important: false,
                },
            ]
        );

        let normal = parse_declarations("row-gap:normal;column-gap:12px");
        assert!(normal.contains(&Declaration {
            property: PropertyId::RowGap,
            value: PropertyValue::Length(0.0),
            important: false,
        }));
        assert!(normal.contains(&Declaration {
            property: PropertyId::ColumnGap,
            value: PropertyValue::Length(12.0),
            important: false,
        }));

        assert!(parse_declarations("gap:-1px").is_empty());
        assert!(parse_declarations("gap:1px 2px 3px").is_empty());

        let mut style = ComputedStyle::default();
        assert_eq!(style.row_gap, 0.0);
        assert_eq!(style.column_gap, 0.0);
        apply_property_value(
            &mut style,
            PropertyId::ColumnGap,
            PropertyValue::Length(14.0),
        );
        assert_eq!(style.column_gap, 14.0);
    }

    #[test]
    fn justify_content_parses_supported_main_axis_values() {
        let declarations = parse_declarations(
            "justify-content:flex-end;justify-content:center;justify-content:space-between;justify-content:space-around;justify-content:space-evenly",
        );
        for value in [
            JustifyContent::FlexEnd,
            JustifyContent::Center,
            JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly,
        ] {
            assert!(declarations.contains(&Declaration {
                property: PropertyId::JustifyContent,
                value: PropertyValue::JustifyContent(value),
                important: false,
            }));
        }

        let normal = parse_declarations("justify-content:NoRmAl");
        assert_eq!(
            normal,
            vec![Declaration {
                property: PropertyId::JustifyContent,
                value: PropertyValue::JustifyContent(JustifyContent::FlexStart),
                important: false,
            }]
        );
        assert!(parse_declarations("justify-content:stretch").is_empty());

        let mut style = ComputedStyle::default();
        assert_eq!(style.justify_content, JustifyContent::FlexStart);
        apply_property_value(
            &mut style,
            PropertyId::JustifyContent,
            PropertyValue::JustifyContent(JustifyContent::SpaceAround),
        );
        assert_eq!(style.justify_content, JustifyContent::SpaceAround);
    }

    #[test]
    fn flex_factors_parse_as_finite_non_negative_numbers() {
        let declarations = parse_declarations("flex-grow:2.5;flex-shrink:0");
        assert!(declarations.contains(&Declaration {
            property: PropertyId::FlexGrow,
            value: PropertyValue::Number(2.5),
            important: false,
        }));
        assert!(declarations.contains(&Declaration {
            property: PropertyId::FlexShrink,
            value: PropertyValue::Number(0.0),
            important: false,
        }));

        let rejected =
            parse_declarations("flex-grow:-1;flex-shrink:NaN;flex-grow:1px;flex-shrink:inf");
        assert!(rejected.is_empty());

        let mut style = ComputedStyle::default();
        apply_property_value(&mut style, PropertyId::FlexGrow, PropertyValue::Number(2.5));
        apply_property_value(
            &mut style,
            PropertyId::FlexShrink,
            PropertyValue::Number(0.25),
        );
        assert_eq!(style.flex_grow, 2.5);
        assert_eq!(style.flex_shrink, 0.25);
    }

    #[test]
    fn display_flex_is_parsed_and_sets_a_distinct_computed_state() {
        let declarations = parse_declarations("display:FlEx");
        assert!(declarations.contains(&Declaration {
            property: PropertyId::Display,
            value: PropertyValue::Display(DisplayValue::Flex),
            important: false,
        }));

        let mut style = ComputedStyle::default();
        apply_property_value(
            &mut style,
            PropertyId::Display,
            PropertyValue::Display(DisplayValue::Flex),
        );
        assert!(style.display_flex);
        assert!(!style.display_none);
        assert!(!style.display_inline);
        assert!(!style.establishes_bfc);
    }

    #[test]
    fn non_finite_lengths_are_rejected() {
        assert_eq!(parse_px("NaNpx"), None);
        assert_eq!(parse_px("infpx"), None);
        assert_eq!(parse_px("-infpx"), None);
        assert_eq!(parse_px("12px"), Some(12.0));
    }
}
