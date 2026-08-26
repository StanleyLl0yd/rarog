use rarog_layout::{Fragment, FragmentKind, FragmentTree};
use rarog_types::{Color, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

pub fn build_display_list(tree: &FragmentTree) -> DisplayList {
    let mut list = DisplayList::default();
    collect(&tree.root, &mut list);
    list
}

fn collect(fragment: &Fragment, list: &mut DisplayList) {
    match fragment.kind {
        FragmentKind::Root => {}
        FragmentKind::Box => {
            if fragment.style.background.a != 0 {
                list.commands.push(DisplayCommand::FillRect {
                    rect: fragment.boxes.border_box,
                    color: fragment.style.background,
                });
            }
            collect_border(fragment, list);
        }
        FragmentKind::Text => {
            let content = fragment.boxes.content_box;
            list.commands.push(DisplayCommand::TextPlaceholder {
                rect: Rect::new(
                    content.origin.x,
                    content.origin.y + 5.0,
                    content.size.width,
                    3.0,
                ),
                color: Color::BLACK,
            });
        }
    }

    for child in &fragment.children {
        collect(child, list);
    }
}

fn collect_border(fragment: &Fragment, list: &mut DisplayList) {
    let widths = fragment.style.border_width;
    let color = fragment.style.border_color;
    if color.a == 0 {
        return;
    }

    let border = fragment.boxes.border_box;
    let x = border.origin.x;
    let y = border.origin.y;
    let width = border.size.width.max(0.0);
    let height = border.size.height.max(0.0);

    push_fill(
        list,
        Rect::new(x, y, width, widths.top.min(height).max(0.0)),
        color,
    );
    push_fill(
        list,
        Rect::new(
            x,
            (y + height - widths.bottom).max(y),
            width,
            widths.bottom.min(height).max(0.0),
        ),
        color,
    );
    push_fill(
        list,
        Rect::new(x, y, widths.left.min(width).max(0.0), height),
        color,
    );
    push_fill(
        list,
        Rect::new(
            (x + width - widths.right).max(x),
            y,
            widths.right.min(width).max(0.0),
            height,
        ),
        color,
    );
}

fn push_fill(list: &mut DisplayList, rect: Rect, color: Color) {
    if rect.size.width > 0.0 && rect.size.height > 0.0 {
        list.commands.push(DisplayCommand::FillRect { rect, color });
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
        for pixel in &self.pixels {
            out.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
        }
        out
    }
}
