use rarog_layout::{LayoutOutput, layout_document};
use rarog_paint::{DisplayList, Framebuffer, build_display_list};
use rarog_types::{Color, Size};

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub viewport: Size,
    pub background: Color,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            viewport: Size {
                width: 1024.0,
                height: 768.0,
            },
            background: Color::WHITE,
        }
    }
}

pub struct RenderOutput {
    pub layout: LayoutOutput,
    pub display_list: DisplayList,
    pub framebuffer: Framebuffer,
}

pub fn render_html(source: &str, options: RenderOptions) -> RenderOutput {
    let document = rarog_html::parse(source);
    let layout = layout_document(&document, options.viewport);
    let display_list = build_display_list(&layout.fragments);
    let mut framebuffer = Framebuffer::new(options.viewport, options.background);
    framebuffer.rasterize(&display_list);

    RenderOutput {
        layout,
        display_list,
        framebuffer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_pipeline_produces_commands_and_fragments() {
        let output = render_html(
            "<html><body><div style=\"background:#ffffff;height:32px\">x</div></body></html>",
            RenderOptions::default(),
        );

        assert!(!output.display_list.commands.is_empty());
        assert!(!output.layout.fragments.root.children.is_empty());
        assert_eq!(output.framebuffer.width, 1024);
        assert_eq!(output.framebuffer.height, 768);
    }

    #[test]
    fn box_model_reaches_paint_without_layout_drawing_directly() {
        let output = render_html(
            "<div style=\"width:100px;height:20px;padding:10px;border-width:2px;\
             border-color:#000000;background:#ffffff\">x</div>",
            RenderOptions {
                viewport: Size {
                    width: 320.0,
                    height: 200.0,
                },
                background: Color::WHITE,
            },
        );

        let fragment = &output.layout.fragments.root.children[0];
        assert_eq!(fragment.boxes.content_box.size.width, 100.0);
        assert_eq!(fragment.boxes.border_box.size.width, 124.0);
        assert!(output.display_list.commands.len() >= 6);
    }
}
