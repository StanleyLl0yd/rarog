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
    pub const ZERO: Self = Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };
    pub fn all(v: f32) -> Self { Self { top: v, right: v, bottom: v, left: v } }
    pub fn horizontal(self) -> f32 { self.left + self.right }
    pub fn vertical(self) -> f32 { self.top + self.bottom }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputedStyle {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub padding: EdgeSizes,
    pub background: Color,
    pub display_none: bool,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self { width: None, height: None, padding: EdgeSizes::ZERO, background: Color::TRANSPARENT, display_none: false }
    }
}

pub fn computed_style(doc: &Document, node: NodeId) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    let NodeKind::Element(el) = &doc.node(node).kind else { return style; };

    if el.tag_name == "body" {
        style.background = Color::WHITE;
        style.padding = EdgeSizes::all(8.0);
    }

    if let Some(inline) = el.attributes.get("style") {
        for decl in inline.split(';') {
            let Some((name, value)) = decl.split_once(':') else { continue; };
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            match name.as_str() {
                "width" => style.width = parse_px(value),
                "height" => style.height = parse_px(value),
                "padding" => if let Some(v) = parse_px(value) { style.padding = EdgeSizes::all(v); },
                "background" | "background-color" => if let Some(c) = parse_color(value) { style.background = c; },
                "display" if value.eq_ignore_ascii_case("none") => style.display_none = true,
                _ => {}
            }
        }
    }
    style
}

fn parse_px(value: &str) -> Option<f32> {
    value.trim().strip_suffix("px").unwrap_or(value.trim()).parse().ok()
}

fn parse_color(value: &str) -> Option<Color> {
    let v = value.trim();
    match v.to_ascii_lowercase().as_str() {
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        _ => {
            let hex = v.strip_prefix('#')?;
            if hex.len() != 6 { return None; }
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
}
