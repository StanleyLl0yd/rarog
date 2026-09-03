from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


selector_path = Path("crates/rarog-css/src/selector.rs")
selector = selector_path.read_text()
selector = replace_once(
    selector,
    "use std::collections::BTreeSet;\n",
    "",
    "selector BTreeSet import",
)
selector = replace_once(
    selector,
    '''        if !self.classes.is_empty() {
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
''',
    '''        if !self.classes.is_empty() {
            let Some(element_classes) = element.attributes.get("class") else {
                return false;
            };
            if self.classes.iter().any(|class| {
                !element_classes
                    .split_whitespace()
                    .any(|candidate| candidate == class)
            }) {
                return false;
            }
        }
''',
    "selector class matching",
)
selector_path.write_text(selector)

css_path = Path("crates/rarog-css/src/lib.rs")
css = css_path.read_text()
css = replace_once(
    css,
    "    style_from_candidates(&candidates, parent_style)\n",
    "    style_from_candidates(candidates, parent_style)\n",
    "cascade caller",
)
css = replace_once(
    css,
    '''fn style_from_candidates(
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
''',
    '''fn style_from_candidates(
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
''',
    "cascade in-place resolution",
)

copy_start = css.index("fn copy_property_from_parent(")
copy_end = css.index("fn apply_property_value(", copy_start)
copy_replacement = '''fn copy_property_from_parent(
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
            style.establishes_bfc = source.establishes_bfc;
        }
        PropertyId::VerticalAlign => style.vertical_align = source.vertical_align,
    }
}

'''
css = css[:copy_start] + copy_replacement + css[copy_end:]

walk_start = css.index("fn collect_style_elements(")
walk_end = css.index("fn parse_declarations(", walk_start)
walk_replacement = '''fn collect_style_elements(document: &Document, node: NodeId, output: &mut Vec<String>) {
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

'''
css = css[:walk_start] + walk_replacement + css[walk_end:]

color_start = css.index("fn parse_color(")
color_end = css.index("\n#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\npub struct DirtyFlags", color_start)
color_replacement = '''fn parse_color(value: &str) -> Option<Color> {
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
                u8::from_str_radix(hex.get(0..2)?, 16).ok()?,
                u8::from_str_radix(hex.get(2..4)?, 16).ok()?,
                u8::from_str_radix(hex.get(4..6)?, 16).ok()?,
            ))
        }
    }
}
'''
css = css[:color_start] + color_replacement + css[color_end:]

subtree_start = css.index("fn mark_subtree(")
subtree_end = css.index("\nfn selector_snapshot(", subtree_start)
subtree_replacement = '''fn mark_subtree(document: &Document, node: NodeId, set: &mut InvalidationSet, flags: DirtyFlags) {
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        set.mark(node, flags);
        if let Some(children) = document.children(node) {
            stack.extend(children.iter().rev().copied());
        }
    }
}
'''
css = css[:subtree_start] + subtree_replacement + css[subtree_end:]

marker = '''    #[test]
    fn parses_css_edge_shorthand() {
'''
test = '''    #[test]
    fn malformed_non_ascii_hex_color_is_rejected_without_panicking() {
        assert_eq!(parse_color("#aéabc"), None);
    }

'''
if marker not in css:
    raise SystemExit("color regression test marker not found")
css = css.replace(marker, test + marker, 1)
css_path.write_text(css)
