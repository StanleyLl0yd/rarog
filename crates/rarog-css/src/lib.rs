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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        Specificity {
            ids: u16::from(self.id.is_some()),
            classes: self.classes.len().min(u16::MAX as usize) as u16,
            types: u16::from(self.tag.is_some()),
        }
    }

    pub fn matches(&self, document: &Document, node: NodeId) -> bool {
        let Some(node) = document.node(node) else {
            return false;
        };
        let NodeKind::Element(element) = &node.kind else {
            return false;
        };

        if let Some(tag) = &self.tag {
            if element.tag_name.as_str() != tag {
                return false;
            }
        }

        if let Some(id) = &self.id {
            if element.attributes.get("id") != Some(id) {
                return false;
            }
        }

        if !self.classes.is_empty() {
            let element_classes = element
                .attributes
                .get("class")
                .map(|value| value.split_whitespace().collect::<BTreeSet<_>>())
                .unwrap_or_default();
            if self
                .classes
                .iter()
                .any(|class| !element_classes.contains(class.as_str()))
            {
                return false;
            }
        }

        true
    }

    pub fn invalidation_key(&self) -> SelectorInvalidationKey {
        SelectorInvalidationKey {
            tag: self.tag.clone(),
            id: self.id.clone(),
            classes: self.classes.iter().cloned().collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    BackgroundColor,
    BorderColor,
    Display,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayValue {
    Block,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyValue {
    Length(f32),
    Color(Color),
    Display(DisplayValue),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Declaration {
    pub property: PropertyId,
    pub value: PropertyValue,
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
        let mut cursor = 0usize;
        let mut source_order = 0u32;

        while cursor < input.len() {
            let Some(open_relative) = input[cursor..].find('{') else {
                break;
            };
            let open = cursor + open_relative;
            let Some(close_relative) = input[open + 1..].find('}') else {
                break;
            };
            let close = open + 1 + close_relative;
            let selector_text = input[cursor..open].trim();
            let declarations = parse_declarations(&input[open + 1..close]);

            if !declarations.is_empty() {
                for selector_text in selector_text.split(',') {
                    if let Some(selector) = parse_selector(selector_text.trim()) {
                        rules.push(StyleRule {
                            specificity: selector.specificity(),
                            selector,
                            declarations: declarations.clone(),
                            source_order,
                            dependencies: Vec::new(),
                        });
                        source_order = source_order.saturating_add(1);
                    }
                }
            }

            cursor = close + 1;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CascadePriority {
    origin: StyleOrigin,
    layer: CascadeLayer,
    specificity: Specificity,
    sheet_order: u32,
    rule_order: u32,
    declaration_order: u32,
}

#[derive(Clone, Copy, Debug)]
struct Winner {
    priority: CascadePriority,
    value: PropertyValue,
}

pub fn computed_style(document: &Document, node: NodeId, styles: &StyleSet) -> ComputedStyle {
    let Some(dom_node) = document.node(node) else {
        return ComputedStyle::default();
    };
    let NodeKind::Element(element) = &dom_node.kind else {
        return ComputedStyle::default();
    };

    let mut winners = BTreeMap::<PropertyId, Winner>::new();

    for (sheet_order, stylesheet) in styles.stylesheets.iter().enumerate() {
        for rule in &stylesheet.rules {
            if !rule.selector.matches(document, node) {
                continue;
            }
            apply_declarations(
                &mut winners,
                &rule.declarations,
                CascadePriority {
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
            &mut winners,
            &declarations,
            CascadePriority {
                origin: StyleOrigin::Inline,
                layer: CascadeLayer(u16::MAX),
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

    style_from_winners(&winners)
}

fn apply_declarations(
    winners: &mut BTreeMap<PropertyId, Winner>,
    declarations: &[Declaration],
    base_priority: CascadePriority,
) {
    for (declaration_order, declaration) in declarations.iter().enumerate() {
        let priority = CascadePriority {
            declaration_order: declaration_order as u32,
            ..base_priority
        };
        let replace = winners
            .get(&declaration.property)
            .map(|winner| priority >= winner.priority)
            .unwrap_or(true);
        if replace {
            winners.insert(
                declaration.property,
                Winner {
                    priority,
                    value: declaration.value,
                },
            );
        }
    }
}

fn style_from_winners(winners: &BTreeMap<PropertyId, Winner>) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    for (property, winner) in winners {
        match (*property, winner.value) {
            (PropertyId::Width, PropertyValue::Length(value)) => {
                style.width = Some(value.max(0.0));
            }
            (PropertyId::Height, PropertyValue::Length(value)) => {
                style.height = Some(value.max(0.0));
            }
            (PropertyId::MarginTop, PropertyValue::Length(value)) => style.margin.top = value,
            (PropertyId::MarginRight, PropertyValue::Length(value)) => style.margin.right = value,
            (PropertyId::MarginBottom, PropertyValue::Length(value)) => style.margin.bottom = value,
            (PropertyId::MarginLeft, PropertyValue::Length(value)) => style.margin.left = value,
            (PropertyId::BorderTopWidth, PropertyValue::Length(value)) => {
                style.border_width.top = value.max(0.0);
            }
            (PropertyId::BorderRightWidth, PropertyValue::Length(value)) => {
                style.border_width.right = value.max(0.0);
            }
            (PropertyId::BorderBottomWidth, PropertyValue::Length(value)) => {
                style.border_width.bottom = value.max(0.0);
            }
            (PropertyId::BorderLeftWidth, PropertyValue::Length(value)) => {
                style.border_width.left = value.max(0.0);
            }
            (PropertyId::PaddingTop, PropertyValue::Length(value)) => {
                style.padding.top = value.max(0.0);
            }
            (PropertyId::PaddingRight, PropertyValue::Length(value)) => {
                style.padding.right = value.max(0.0);
            }
            (PropertyId::PaddingBottom, PropertyValue::Length(value)) => {
                style.padding.bottom = value.max(0.0);
            }
            (PropertyId::PaddingLeft, PropertyValue::Length(value)) => {
                style.padding.left = value.max(0.0);
            }
            (PropertyId::BackgroundColor, PropertyValue::Color(color)) => {
                style.background = color;
            }
            (PropertyId::BorderColor, PropertyValue::Color(color)) => {
                style.border_color = color;
            }
            (PropertyId::Display, PropertyValue::Display(display)) => {
                style.display_none = display == DisplayValue::None;
            }
            _ => {}
        }
    }
    style
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

pub fn parse_selector(input: &str) -> Option<Selector> {
    let input = input.trim();
    if input.is_empty()
        || input.chars().any(char::is_whitespace)
        || input.contains([':', '[', ']', '>', '+', '~'])
    {
        return None;
    }

    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    let mut tag = None;
    let mut id = None;
    let mut classes = Vec::new();

    if bytes.first().copied() == Some(b'*') {
        cursor = 1;
    } else if bytes
        .first()
        .is_some_and(|byte| *byte != b'.' && *byte != b'#')
    {
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'.' && bytes[cursor] != b'#' {
            cursor += 1;
        }
        let value = input[start..cursor].trim();
        if value.is_empty() {
            return None;
        }
        tag = Some(value.to_ascii_lowercase());
    }

    while cursor < bytes.len() {
        let marker = bytes[cursor];
        if marker != b'.' && marker != b'#' {
            return None;
        }
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'.' && bytes[cursor] != b'#' {
            cursor += 1;
        }
        if start == cursor {
            return None;
        }
        let value = input[start..cursor].to_string();
        if marker == b'#' {
            if id.replace(value).is_some() {
                return None;
            }
        } else {
            classes.push(value);
        }
    }

    Some(Selector { tag, id, classes })
}

fn parse_declarations(input: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for declaration in input.split(';') {
        let Some((name, value)) = declaration.split_once(':') else {
            continue;
        };
        append_property(
            &mut declarations,
            name.trim().to_ascii_lowercase().as_str(),
            value.trim(),
        );
    }
    declarations
}

fn append_property(output: &mut Vec<Declaration>, name: &str, value: &str) {
    match name {
        "width" => push_length(output, PropertyId::Width, value, false),
        "height" => push_length(output, PropertyId::Height, value, false),
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
        ),
        "margin-top" => push_length(output, PropertyId::MarginTop, value, true),
        "margin-right" => push_length(output, PropertyId::MarginRight, value, true),
        "margin-bottom" => push_length(output, PropertyId::MarginBottom, value, true),
        "margin-left" => push_length(output, PropertyId::MarginLeft, value, true),
        "padding-top" => push_length(output, PropertyId::PaddingTop, value, false),
        "padding-right" => push_length(output, PropertyId::PaddingRight, value, false),
        "padding-bottom" => push_length(output, PropertyId::PaddingBottom, value, false),
        "padding-left" => push_length(output, PropertyId::PaddingLeft, value, false),
        "border-top-width" => push_length(output, PropertyId::BorderTopWidth, value, false),
        "border-right-width" => push_length(output, PropertyId::BorderRightWidth, value, false),
        "border-bottom-width" => push_length(output, PropertyId::BorderBottomWidth, value, false),
        "border-left-width" => push_length(output, PropertyId::BorderLeftWidth, value, false),
        "background" | "background-color" => {
            if let Some(color) = parse_color(value) {
                output.push(Declaration {
                    property: PropertyId::BackgroundColor,
                    value: PropertyValue::Color(color),
                });
            }
        }
        "border-color" => {
            if let Some(color) = parse_color(value) {
                output.push(Declaration {
                    property: PropertyId::BorderColor,
                    value: PropertyValue::Color(color),
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
                });
            }
        }
        _ => {}
    }
}

fn push_length(output: &mut Vec<Declaration>, property: PropertyId, value: &str, negative: bool) {
    if let Some(mut value) = parse_px(value) {
        if !negative {
            value = value.max(0.0);
        }
        output.push(Declaration {
            property,
            value: PropertyValue::Length(value),
        });
    }
}

fn push_edges(
    output: &mut Vec<Declaration>,
    value: &str,
    properties: [PropertyId; 4],
    negative: bool,
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
                        mark_subtree(document, *child, &mut set, DirtyFlags::STYLE_LAYOUT_PAINT);
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
                        let parent = document.node(*node).and_then(|node| node.parent);
                        set.mark_ancestors(document, parent, DirtyFlags::LAYOUT_PAINT);
                    }
                    if matches!(name.as_str(), "id" | "class") {
                        set.mark_relational_dependents(document, *node, name, dependencies);
                    }
                }
                MutationKind::CharacterData { node } => {
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
    let mut output = selector.tag.clone().unwrap_or_else(|| "*".into());
    if let Some(id) = &selector.id {
        output.push('#');
        output.push_str(id);
    }
    for class in &selector.classes {
        output.push('.');
        output.push_str(class);
    }
    output
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
    fn non_finite_lengths_are_rejected() {
        assert_eq!(parse_px("NaNpx"), None);
        assert_eq!(parse_px("infpx"), None);
        assert_eq!(parse_px("-infpx"), None);
        assert_eq!(parse_px("12px"), Some(12.0));
    }
}
