use rarog_css::StyleSet;
use rarog_dom::Document;
use rarog_layout::{LayoutOutput, layout_document_with_styles};
use rarog_paint::{DamageRegion, DisplayList, Framebuffer, build_display_list};
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
    pub document: Document,
    pub styles: StyleSet,
    pub layout: LayoutOutput,
    pub display_list: DisplayList,
    pub damage: DamageRegion,
    pub framebuffer: Framebuffer,
}

impl RenderOutput {
    pub fn deterministic_signature_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        hash = fnv1a(hash, self.document.snapshot().as_bytes());
        hash = fnv1a(hash, self.styles.snapshot().as_bytes());
        hash = fnv1a(hash, self.layout.tree.style_snapshot().as_bytes());
        hash = fnv1a(hash, self.layout.tree.snapshot().as_bytes());
        hash = fnv1a(hash, self.layout.fragments.snapshot().as_bytes());
        hash = fnv1a(hash, self.display_list.snapshot().as_bytes());
        fnv1a(hash, &self.framebuffer.stable_hash64().to_le_bytes())
    }
}

pub fn render_html(source: &str, options: RenderOptions) -> RenderOutput {
    render_html_against(source, options, None)
}

pub fn render_html_against(
    source: &str,
    options: RenderOptions,
    previous_display_list: Option<&DisplayList>,
) -> RenderOutput {
    let document = rarog_html::parse(source);
    let styles = StyleSet::for_document(&document);
    let layout = layout_document_with_styles(&document, &styles, options.viewport);
    let display_list = build_display_list(&layout.fragments);
    let damage = DamageRegion::between(previous_display_list, &display_list);
    let mut framebuffer = Framebuffer::new(options.viewport, options.background);
    framebuffer.rasterize(&display_list);

    RenderOutput {
        document,
        styles,
        layout,
        display_list,
        damage,
        framebuffer,
    }
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    const DETERMINISTIC_FIXTURE: &str = "<style>.card { width:80px; padding:4px; background:#112233; } #hero { border-width:2px; border-color:#000000; }</style><div id=\"hero\" class=\"card\">Rarog</div>";

    fn deterministic_options() -> RenderOptions {
        RenderOptions {
            viewport: Size {
                width: 160.0,
                height: 90.0,
            },
            background: Color::WHITE,
        }
    }

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

    #[test]
    fn author_stylesheet_cascade_reaches_rendering() {
        let output = render_html(DETERMINISTIC_FIXTURE, deterministic_options());
        let fragment = &output.layout.fragments.root.children[0];

        assert_eq!(fragment.boxes.content_box.size.width, 80.0);
        assert_eq!(fragment.style.background, Color::rgb(0x11, 0x22, 0x33));
        assert_eq!(fragment.style.border_width.top, 2.0);
    }

    #[test]
    fn damage_is_empty_when_display_list_is_unchanged() {
        let first = render_html(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_html_against(
            DETERMINISTIC_FIXTURE,
            deterministic_options(),
            Some(&first.display_list),
        );

        assert!(second.damage.rects.is_empty());
    }

    #[test]
    fn deterministic_render_snapshot_and_hash() {
        let first = render_html(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_html(DETERMINISTIC_FIXTURE, deterministic_options());

        assert_eq!(first.document.snapshot(), second.document.snapshot());
        assert_eq!(first.styles.snapshot(), second.styles.snapshot());
        assert_eq!(
            first.layout.tree.style_snapshot(),
            second.layout.tree.style_snapshot()
        );
        assert_eq!(first.layout.tree.snapshot(), second.layout.tree.snapshot());
        assert_eq!(
            first.layout.fragments.snapshot(),
            second.layout.fragments.snapshot()
        );
        assert_eq!(first.display_list.snapshot(), second.display_list.snapshot());
        assert_eq!(
            first.framebuffer.stable_hash64(),
            second.framebuffer.stable_hash64()
        );
        assert_eq!(
            first.deterministic_signature_hash(),
            second.deterministic_signature_hash()
        );

        assert_eq!(first.framebuffer.stable_hash64(), 0);
        assert_eq!(first.deterministic_signature_hash(), 0);
    }
}
