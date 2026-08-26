use rarog_dom::{Document, NodeId, NodeKind};
use rarog_types::Color;

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

pub fn computed_style(doc: &Document, node: NodeId) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    let NodeKind::Element(element) = &doc.node(node).kind else {
        return style;
    };

    if element.tag_name == "body" {
        style.background = Color::WHITE;
        style.margin = EdgeSizes::all(8.0);
    }

    if let Some(inline) = element.attributes.get("style") {
        for declaration in inline.split(';') {
            let Some((name, value)) = declaration.split_once(':') else {
                continue;
            };
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();

            match name.as_str() {
                "width" => style.width = parse_px(value).map(|value| value.max(0.0)),
                "height" => style.height = parse_px(value).map(|value| value.max(0.0)),
                "margin" => {
                    if let Some(edges) = parse_edge_sizes(value) {
                        style.margin = edges;
                    }
                }
                "padding" => {
                    if let Some(edges) = parse_edge_sizes(value) {
                        style.padding = edges.non_negative();
                    }
                }
                "border-width" => {
                    if let Some(edges) = parse_edge_sizes(value) {
                        style.border_width = edges.non_negative();
                    }
                }
                "margin-top" => set_edge(&mut style.margin.top, value, true),
                "margin-right" => set_edge(&mut style.margin.right, value, true),
                "margin-bottom" => set_edge(&mut style.margin.bottom, value, true),
                "margin-left" => set_edge(&mut style.margin.left, value, true),
                "padding-top" => set_edge(&mut style.padding.top, value, false),
                "padding-right" => set_edge(&mut style.padding.right, value, false),
                "padding-bottom" => set_edge(&mut style.padding.bottom, value, false),
                "padding-left" => set_edge(&mut style.padding.left, value, false),
                "border-top-width" => set_edge(&mut style.border_width.top, value, false),
                "border-right-width" => set_edge(&mut style.border_width.right, value, false),
                "border-bottom-width" => set_edge(&mut style.border_width.bottom, value, false),
                "border-left-width" => set_edge(&mut style.border_width.left, value, false),
                "background" | "background-color" => {
                    if let Some(color) = parse_color(value) {
                        style.background = color;
                    }
                }
                "border-color" => {
                    if let Some(color) = parse_color(value) {
                        style.border_color = color;
                    }
                }
                "display" if value.eq_ignore_ascii_case("none") => style.display_none = true,
                _ => {}
            }
        }
    }

    style
}

fn set_edge(edge: &mut f32, value: &str, allow_negative: bool) {
    if let Some(parsed) = parse_px(value) {
        *edge = if allow_negative {
            parsed
        } else {
            parsed.max(0.0)
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_color() {
        assert_eq!(parse_color("#112233"), Some(Color::rgb(0x11, 0x22, 0x33)));
    }

    #[test]
    fn parses_pixel_value() {
        assert_eq!(parse_px("24px"), Some(24.0));
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
}
