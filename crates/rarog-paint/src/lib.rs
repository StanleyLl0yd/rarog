use rarog_layout::LayoutBox;
use rarog_types::{Color, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

pub fn build_display_list(layout: &LayoutBox) -> DisplayList {
    let mut list = DisplayList::default();
    collect(layout, &mut list);
    list
}

fn collect(layout: &LayoutBox, list: &mut DisplayList) {
    if layout.style.background.a != 0 {
        list.commands.push(DisplayCommand::FillRect {
            rect: layout.rect,
            color: layout.style.background,
        });
    }
    if layout.children.is_empty()
        && layout.style.background.a == 0
        && layout.rect.size.height <= 18.0
    {
        list.commands.push(DisplayCommand::TextPlaceholder {
            rect: Rect::new(
                layout.rect.origin.x,
                layout.rect.origin.y + 5.0,
                layout.rect.size.width,
                3.0,
            ),
            color: Color::BLACK,
        });
    }
    for child in &layout.children {
        collect(child, list);
    }
}

#[derive(Clone, Debug)]
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<Color>,
}

impl Framebuffer {
    pub fn new(size: Size, background: Color) -> Self {
        let width = size.width.max(1.0).round() as u32;
        let height = size.height.max(1.0).round() as u32;
        Self {
            width,
            height,
            pixels: vec![background; (width * height) as usize],
        }
    }

    pub fn rasterize(&mut self, list: &DisplayList) {
        for command in &list.commands {
            match *command {
                DisplayCommand::FillRect { rect, color }
                | DisplayCommand::TextPlaceholder { rect, color } => self.fill_rect(rect, color),
            }
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x0 = rect.origin.x.floor().max(0.0) as u32;
        let y0 = rect.origin.y.floor().max(0.0) as u32;
        let x1 = (rect.origin.x + rect.size.width)
            .ceil()
            .clamp(0.0, self.width as f32) as u32;
        let y1 = (rect.origin.y + rect.size.height)
            .ceil()
            .clamp(0.0, self.height as f32) as u32;
        for y in y0..y1 {
            for x in x0..x1 {
                self.pixels[(y * self.width + x) as usize] = color;
            }
        }
    }

    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for p in &self.pixels {
            out.extend_from_slice(&[p.r, p.g, p.b]);
        }
        out
    }
}
