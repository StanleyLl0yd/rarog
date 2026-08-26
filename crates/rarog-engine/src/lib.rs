use rarog_layout::{LayoutBox, layout_document};
use rarog_paint::{DisplayList, Framebuffer, build_display_list};
use rarog_types::{Color, Size};

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub viewport: Size,
    pub background: Color,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { viewport: Size { width: 1024.0, height: 768.0 }, background: Color::WHITE }
    }
}

pub struct RenderOutput {
    pub layout: LayoutBox,
    pub display_list: DisplayList,
    pub framebuffer: Framebuffer,
}

pub fn render_html(source: &str, options: RenderOptions) -> RenderOutput {
    let document = rarog_html::parse(source);
    let layout = layout_document(&document, options.viewport);
    let display_list = build_display_list(&layout);
    let mut framebuffer = Framebuffer::new(options.viewport, options.background);
    framebuffer.rasterize(&display_list);
    RenderOutput { layout, display_list, framebuffer }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_pipeline_produces_commands() {
        let output = render_html(
            "<html><body><div style=\"background:#ffffff;height:32px\">x</div></body></html>",
            RenderOptions::default(),
        );
        assert!(!output.display_list.commands.is_empty());
        assert_eq!(output.framebuffer.width, 1024);
        assert_eq!(output.framebuffer.height, 768);
    }
}
