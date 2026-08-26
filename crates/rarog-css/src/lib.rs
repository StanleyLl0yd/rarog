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
        let NodeKind::Element(element) = &document.node(node).kind else {
            return false;
        };

        if let Some(tag) = &self.tag {
            if &element.tag_name != tag {
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
    let NodeKind::Element(element) = &document.node(node).kind else {
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
    if let NodeKind::Element(element) = &document.node(node).kind {
        if element.tag_name == "style" {
            let mut text = String::new();
            collect_text(document, node, &mut text);
            if !text.trim().is_empty() {
                output.push(text);
            }
            return;
        }
    }

    for child in document.children(node) {
        collect_style_elements(document, *child, output);
    }
}

fn collect_text(document: &Document, node: NodeId, output: &mut String) {
    for child in document.children(node) {
        match &document.node(*child).kind {
            NodeKind::Text(text) => {
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
    } else if bytes.first().is_some_and(|byte| *byte != b'.' && *byte != b'#') {
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
    value
        .trim()
        .strip_suffix("px")
        .unwrap_or(value.trim())
        .parse()
        .ok()
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
            node = document.node(current).parent;
        }
    }
}

fn mark_subtree(
    document: &Document,
    node: NodeId,
    set: &mut InvalidationSet,
    flags: DirtyFlags,
) {
    set.mark(node, flags);
    for child in document.children(node) {
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

    fn document_with_element(
        tag: &str,
        attributes: &[(&str, &str)],
    ) -> (Document, NodeId) {
        let mut document = Document::new();
        let mut attrs = BTreeMap::new();
        for (name, value) in attributes {
            attrs.insert((*name).into(), (*value).into());
        }
        let node = document
            .append_new(
                document.root(),
                NodeKind::Element(ElementData {
                    tag_name: tag.into(),
                    attributes: attrs,
                }),
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
                NodeKind::Element(ElementData {
                    tag_name: "style".into(),
                    attributes: BTreeMap::new(),
                }),
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
                NodeKind::Element(ElementData {
                    tag_name: "div".into(),
                    attributes,
                }),
            )
            .unwrap();
        (document, target)
    }
}
