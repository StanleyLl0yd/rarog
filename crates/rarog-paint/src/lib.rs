use rarog_layout::{Fragment, FragmentKind, FragmentTree};
use rarog_types::{Color, Rect, Size};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_FRAMEBUFFER_PIXELS: u64 = 67_108_864;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplayItemId {
    pub source: u64,
    pub fragment: u64,
    pub slot: u8,
}

impl DisplayItemId {
    #[cfg(test)]
    const fn test(value: u64) -> Self {
        Self {
            source: value,
            fragment: value,
            slot: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
    PushClip { rect: Rect },
    PopClip,
}

impl DisplayCommand {
    pub fn bounds(self) -> Option<Rect> {
        match self {
            Self::FillRect { rect, .. }
            | Self::TextPlaceholder { rect, .. }
            | Self::PushClip { rect } => Some(rect),
            Self::PopClip => None,
        }
    }

    fn is_clip(self) -> bool {
        matches!(self, Self::PushClip { .. } | Self::PopClip)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub command_ids: Vec<DisplayItemId>,
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    fn push(&mut self, id: DisplayItemId, command: DisplayCommand) {
        self.command_ids.push(id);
        self.commands.push(command);
    }

    pub fn has_unique_ids(&self) -> bool {
        self.command_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            == self.command_ids.len()
    }

    pub fn snapshot(&self) -> String {
        let mut output = String::new();
        for (id, command) in self.command_ids.iter().zip(&self.commands) {
            match command {
                DisplayCommand::FillRect { rect, color } => output.push_str(&format!(
                    "{}|fill|{}|{:02x}{:02x}{:02x}{:02x}\n",
                    display_item_id_snapshot(*id),
                    rect_snapshot(*rect),
                    color.r,
                    color.g,
                    color.b,
                    color.a
                )),
                DisplayCommand::TextPlaceholder { rect, color } => output.push_str(&format!(
                    "{}|text-placeholder|{}|{:02x}{:02x}{:02x}{:02x}\n",
                    display_item_id_snapshot(*id),
                    rect_snapshot(*rect),
                    color.r,
                    color.g,
                    color.b,
                    color.a
                )),
                DisplayCommand::PushClip { rect } => output.push_str(&format!(
                    "{}|push-clip|{}\n",
                    display_item_id_snapshot(*id),
                    rect_snapshot(*rect)
                )),
                DisplayCommand::PopClip => {
                    output.push_str(&format!("{}|pop-clip\n", display_item_id_snapshot(*id)))
                }
            }
        }
        output
    }
}

pub fn build_display_list(tree: &FragmentTree) -> DisplayList {
    build_display_list_for_fragment(&tree.root)
}

pub fn build_display_list_for_fragment(fragment: &Fragment) -> DisplayList {
    let mut list = DisplayList::default();
    collect(fragment, &mut list);
    assert!(list.has_unique_ids(), "display item IDs must be unique");
    list
}

pub fn replace_display_items_for_fragment(
    list: &mut DisplayList,
    previous: &Fragment,
    current: &Fragment,
) -> bool {
    let previous_items = build_display_list_for_fragment(previous);
    let current_items = build_display_list_for_fragment(current);
    replace_display_items(list, &previous_items, &current_items)
}

fn replace_display_items(
    list: &mut DisplayList,
    previous: &DisplayList,
    current: &DisplayList,
) -> bool {
    if previous.command_ids.is_empty() {
        return current.command_ids.is_empty();
    }

    let removed = previous
        .command_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if !list.command_ids.iter().any(|id| removed.contains(id)) {
        return false;
    }

    let mut command_ids = Vec::with_capacity(
        list.command_ids.len() - previous.command_ids.len().min(list.command_ids.len())
            + current.command_ids.len(),
    );
    let mut commands = Vec::with_capacity(command_ids.capacity());
    let mut inserted = false;

    for (id, command) in list
        .command_ids
        .iter()
        .copied()
        .zip(list.commands.iter().copied())
    {
        if removed.contains(&id) {
            if !inserted {
                command_ids.extend_from_slice(&current.command_ids);
                commands.extend_from_slice(&current.commands);
                inserted = true;
            }
            continue;
        }
        command_ids.push(id);
        commands.push(command);
    }

    if !inserted {
        return false;
    }
    list.command_ids = command_ids;
    list.commands = commands;
    true
}

fn collect(fragment: &Fragment, list: &mut DisplayList) {
    match fragment.kind {
        FragmentKind::Root => {}
        FragmentKind::Box => {
            if fragment.style.background.a != 0 {
                list.push(
                    item_id(fragment, 0),
                    DisplayCommand::FillRect {
                        rect: fragment.boxes.border_box,
                        color: fragment.style.background,
                    },
                );
            }
            collect_border(fragment, list);
        }
        FragmentKind::Text => {
            let content = fragment.boxes.content_box;
            list.push(
                item_id(fragment, 0),
                DisplayCommand::TextPlaceholder {
                    rect: Rect::new(
                        content.origin.x,
                        content.origin.y + 5.0,
                        content.size.width,
                        3.0,
                    ),
                    color: Color::BLACK,
                },
            );
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
        item_id(fragment, 1),
        Rect::new(x, y, width, widths.top.min(height).max(0.0)),
        color,
    );
    push_fill(
        list,
        item_id(fragment, 2),
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
        item_id(fragment, 3),
        Rect::new(x, y, widths.left.min(width).max(0.0), height),
        color,
    );
    push_fill(
        list,
        item_id(fragment, 4),
        Rect::new(
            (x + width - widths.right).max(x),
            y,
            widths.right.min(width).max(0.0),
            height,
        ),
        color,
    );
}

fn item_id(fragment: &Fragment, slot: u8) -> DisplayItemId {
    let source = fragment
        .dom_node
        .map(|node| node as u64)
        .unwrap_or_else(|| (1_u64 << 63) | fragment.layout_node.index() as u64);
    DisplayItemId {
        source,
        fragment: fragment.id.index() as u64,
        slot,
    }
}

fn display_item_id_snapshot(id: DisplayItemId) -> String {
    format!("{}:{}:{}", id.source, id.fragment, id.slot)
}

fn push_fill(list: &mut DisplayList, id: DisplayItemId, rect: Rect, color: Color) {
    if rect.size.width > 0.0 && rect.size.height > 0.0 {
        list.push(id, DisplayCommand::FillRect { rect, color });
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageRegion {
    pub rects: Vec<Rect>,
}

impl DamageRegion {
    pub fn between(previous: Option<&DisplayList>, current: &DisplayList) -> Self {
        assert!(
            current.has_unique_ids(),
            "current display list contains duplicate display item IDs"
        );
        if let Some(previous) = previous {
            assert!(
                previous.has_unique_ids(),
                "previous display list contains duplicate display item IDs"
            );
        }

        let Some(previous) = previous else {
            let mut damage = Self::default();
            for command in &current.commands {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            return damage;
        };

        if previous
            .commands
            .iter()
            .copied()
            .any(DisplayCommand::is_clip)
            || current
                .commands
                .iter()
                .copied()
                .any(DisplayCommand::is_clip)
        {
            let mut damage = Self::default();
            for command in previous.commands.iter().chain(&current.commands) {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            return damage;
        }

        let previous_items = indexed_commands(previous);
        let current_items = indexed_commands(current);
        let ids = previous_items
            .keys()
            .chain(current_items.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let mut damage = Self::default();

        for id in ids {
            let before = previous_items.get(&id).copied();
            let after = current_items.get(&id).copied();
            if before == after {
                continue;
            }
            if let Some(command) = before {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            if let Some(command) = after {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
        }

        damage
    }

    fn push(&mut self, rect: Rect) {
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        if !self.rects.contains(&rect) {
            self.rects.push(rect);
        }
    }
}

fn indexed_commands(list: &DisplayList) -> BTreeMap<DisplayItemId, DisplayCommand> {
    list.command_ids
        .iter()
        .copied()
        .zip(list.commands.iter().copied())
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferError {
    NonFiniteSize,
    DimensionsTooLarge,
    PixelCountOverflow,
    PixelLimitExceeded { pixels: u64, limit: u64 },
}

impl fmt::Display for FramebufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteSize => formatter.write_str("framebuffer size must be finite"),
            Self::DimensionsTooLarge => formatter.write_str("framebuffer dimensions exceed u32"),
            Self::PixelCountOverflow => formatter.write_str("framebuffer pixel count overflow"),
            Self::PixelLimitExceeded { pixels, limit } => {
                write!(
                    formatter,
                    "framebuffer requires {pixels} pixels; limit is {limit}"
                )
            }
        }
    }
}

impl std::error::Error for FramebufferError {}

#[derive(Clone, Debug)]
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<Color>,
}

impl Framebuffer {
    pub fn new(size: Size, background: Color) -> Self {
        Self::try_new(size, background)
            .expect("framebuffer dimensions must fit the R0 safety budget")
    }

    pub fn try_new(size: Size, background: Color) -> Result<Self, FramebufferError> {
        if !size.width.is_finite() || !size.height.is_finite() {
            return Err(FramebufferError::NonFiniteSize);
        }

        let width = size.width.max(1.0).round();
        let height = size.height.max(1.0).round();
        if width > u32::MAX as f32 || height > u32::MAX as f32 {
            return Err(FramebufferError::DimensionsTooLarge);
        }

        let width = width as u32;
        let height = height as u32;
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or(FramebufferError::PixelCountOverflow)?;
        if pixels > MAX_FRAMEBUFFER_PIXELS {
            return Err(FramebufferError::PixelLimitExceeded {
                pixels,
                limit: MAX_FRAMEBUFFER_PIXELS,
            });
        }
        let length = usize::try_from(pixels).map_err(|_| FramebufferError::PixelCountOverflow)?;

        Ok(Self {
            width,
            height,
            pixels: vec![background; length],
        })
    }

    pub fn rasterize(&mut self, list: &DisplayList) {
        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);
        let mut clips = vec![framebuffer_clip];
        for command in &list.commands {
            match *command {
                DisplayCommand::FillRect { rect, color }
                | DisplayCommand::TextPlaceholder { rect, color } => {
                    if let Some(clipped) = intersection(rect, *clips.last().expect("clip stack")) {
                        self.fill_rect(clipped, color);
                    }
                }
                DisplayCommand::PushClip { rect } => {
                    let current = *clips.last().expect("clip stack");
                    clips
                        .push(intersection(current, rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)));
                }
                DisplayCommand::PopClip => {
                    if clips.len() > 1 {
                        clips.pop();
                    }
                }
            }
        }
    }

    pub fn rasterize_damage(
        &mut self,
        list: &DisplayList,
        damage: &DamageRegion,
        background: Color,
    ) {
        if list.commands.iter().copied().any(DisplayCommand::is_clip) {
            self.fill_rect(
                Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
                background,
            );
            self.rasterize(list);
            return;
        }
        for damaged in &damage.rects {
            self.fill_rect(*damaged, background);
            for command in &list.commands {
                let (rect, color) = match *command {
                    DisplayCommand::FillRect { rect, color }
                    | DisplayCommand::TextPlaceholder { rect, color } => (rect, color),
                    DisplayCommand::PushClip { .. } | DisplayCommand::PopClip => continue,
                };
                if let Some(clipped) = intersection(rect, *damaged) {
                    self.fill_rect(clipped, color);
                }
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

    pub fn stable_hash64(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        hash = fnv1a(hash, &self.width.to_le_bytes());
        hash = fnv1a(hash, &self.height.to_le_bytes());
        for pixel in &self.pixels {
            hash = fnv1a(hash, &[pixel.r, pixel.g, pixel.b, pixel.a]);
        }
        hash
    }

    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for pixel in &self.pixels {
            out.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
        }
        out
    }
}

fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.origin.x.max(b.origin.x);
    let y0 = a.origin.y.max(b.origin.y);
    let x1 = (a.origin.x + a.size.width).min(b.origin.x + b.size.width);
    let y1 = (a.origin.y + a.size.height).min(b.origin.y + b.size.height);
    if x1 <= x0 || y1 <= y0 {
        None
    } else {
        Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn rect_snapshot(rect: Rect) -> String {
    format!(
        "{:.1},{:.1},{:.1},{:.1}",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_fill(id: u64, rect: Rect, color: Color) -> DisplayList {
        DisplayList {
            command_ids: vec![DisplayItemId::test(id)],
            commands: vec![DisplayCommand::FillRect { rect, color }],
        }
    }

    #[test]
    fn damage_tracks_changed_display_items() {
        let before = single_fill(1, Rect::new(0.0, 0.0, 10.0, 10.0), Color::BLACK);
        let after = single_fill(1, Rect::new(5.0, 0.0, 10.0, 10.0), Color::BLACK);
        let damage = DamageRegion::between(Some(&before), &after);

        assert_eq!(damage.rects.len(), 2);
        assert!(damage.rects.contains(&Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert!(damage.rects.contains(&Rect::new(5.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn unchanged_display_list_has_no_damage() {
        let list = single_fill(1, Rect::new(0.0, 0.0, 10.0, 10.0), Color::BLACK);
        assert!(DamageRegion::between(Some(&list), &list).rects.is_empty());
    }

    #[test]
    fn retained_display_patch_preserves_unrelated_items() {
        let first = single_fill(1, Rect::new(0.0, 0.0, 4.0, 4.0), Color::BLACK);
        let middle = single_fill(2, Rect::new(4.0, 0.0, 4.0, 4.0), Color::BLACK);
        let last = single_fill(3, Rect::new(8.0, 0.0, 4.0, 4.0), Color::BLACK);
        let mut list = DisplayList {
            command_ids: vec![
                first.command_ids[0],
                middle.command_ids[0],
                last.command_ids[0],
            ],
            commands: vec![first.commands[0], middle.commands[0], last.commands[0]],
        };
        let replacement = single_fill(2, Rect::new(5.0, 0.0, 4.0, 4.0), Color::BLACK);

        assert!(replace_display_items(&mut list, &middle, &replacement));
        assert_eq!(
            list.command_ids,
            vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3)
            ]
        );
        assert_eq!(list.commands[0], first.commands[0]);
        assert_eq!(list.commands[2], last.commands[0]);
        assert_eq!(list.commands[1], replacement.commands[0]);
    }

    #[test]
    fn damage_raster_matches_full_raster() {
        let before = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(2)],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(4.0, 0.0, 4.0, 4.0),
                    color: Color::rgb(10, 20, 30),
                },
            ],
        };
        let after = DisplayList {
            command_ids: before.command_ids.clone(),
            commands: vec![
                before.commands[0],
                DisplayCommand::FillRect {
                    rect: Rect::new(5.0, 0.0, 3.0, 4.0),
                    color: Color::rgb(30, 20, 10),
                },
            ],
        };
        let damage = DamageRegion::between(Some(&before), &after);
        let size = Size {
            width: 8.0,
            height: 4.0,
        };
        let mut incremental = Framebuffer::new(size, Color::WHITE);
        incremental.rasterize(&before);
        incremental.rasterize_damage(&after, &damage, Color::WHITE);
        let mut full = Framebuffer::new(size, Color::WHITE);
        full.rasterize(&after);

        assert_eq!(incremental.stable_hash64(), full.stable_hash64());
    }

    #[test]
    fn framebuffer_rejects_unbounded_allocations() {
        let result = Framebuffer::try_new(
            Size {
                width: 100_000.0,
                height: 100_000.0,
            },
            Color::WHITE,
        );
        assert!(matches!(
            result,
            Err(FramebufferError::PixelLimitExceeded { .. })
        ));
    }

    #[test]
    fn framebuffer_hash_is_stable() {
        let list = single_fill(1, Rect::new(0.0, 0.0, 2.0, 2.0), Color::BLACK);
        let mut framebuffer = Framebuffer::new(
            Size {
                width: 4.0,
                height: 4.0,
            },
            Color::WHITE,
        );
        framebuffer.rasterize(&list);

        assert_eq!(framebuffer.stable_hash64(), framebuffer.stable_hash64());
    }
}

#[cfg(test)]
mod display_identity_hardening_tests {
    use super::*;

    #[test]
    fn fragment_component_prevents_multi_fragment_collisions() {
        let first = DisplayItemId {
            source: 7,
            fragment: 10,
            slot: 0,
        };
        let second = DisplayItemId {
            source: 7,
            fragment: 11,
            slot: 0,
        };
        assert_ne!(first, second);
    }

    #[test]
    fn duplicate_ids_are_detectable() {
        let id = DisplayItemId {
            source: 1,
            fragment: 2,
            slot: 3,
        };
        let list = DisplayList {
            command_ids: vec![id, id],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        assert!(!list.has_unique_ids());
    }
    #[test]
    #[should_panic(expected = "current display list contains duplicate display item IDs")]
    fn damage_rejects_duplicate_display_ids() {
        let id = DisplayItemId {
            source: 1,
            fragment: 2,
            slot: 0,
        };
        let list = DisplayList {
            command_ids: vec![id, id],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        let _ = DamageRegion::between(None, &list);
    }

    #[test]
    fn nested_clip_commands_constrain_rasterization() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(1),
            DisplayCommand::PushClip {
                rect: Rect::new(1.0, 1.0, 4.0, 4.0),
            },
        );
        list.push(
            DisplayItemId::test(2),
            DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 8.0, 8.0),
                color: Color::BLACK,
            },
        );
        list.push(
            DisplayItemId::test(3),
            DisplayCommand::PushClip {
                rect: Rect::new(3.0, 0.0, 4.0, 4.0),
            },
        );
        list.push(
            DisplayItemId::test(4),
            DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 8.0, 8.0),
                color: Color::rgb(255, 0, 0),
            },
        );
        list.push(DisplayItemId::test(5), DisplayCommand::PopClip);
        list.push(DisplayItemId::test(6), DisplayCommand::PopClip);

        let mut framebuffer = Framebuffer::new(
            Size {
                width: 8.0,
                height: 8.0,
            },
            Color::WHITE,
        );
        framebuffer.rasterize(&list);

        assert_eq!(framebuffer.pixels[(2 * 8 + 2) as usize], Color::BLACK);
        assert_eq!(
            framebuffer.pixels[(2 * 8 + 3) as usize],
            Color::rgb(255, 0, 0)
        );
        assert_eq!(framebuffer.pixels[0], Color::WHITE);
        assert_eq!(framebuffer.pixels[(6 * 8 + 6) as usize], Color::WHITE);
    }

    #[test]
    fn clip_display_commands_have_deterministic_snapshots() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(7),
            DisplayCommand::PushClip {
                rect: Rect::new(1.0, 2.0, 3.0, 4.0),
            },
        );
        list.push(DisplayItemId::test(8), DisplayCommand::PopClip);
        assert_eq!(
            list.snapshot(),
            "7:7:0|push-clip|1.0,2.0,3.0,4.0\n8:8:0|pop-clip\n"
        );
    }

    #[test]
    fn damage_raster_with_clips_matches_full_raster() {
        let mut previous = DisplayList::default();
        previous.push(
            DisplayItemId::test(1),
            DisplayCommand::PushClip {
                rect: Rect::new(1.0, 1.0, 4.0, 4.0),
            },
        );
        previous.push(
            DisplayItemId::test(2),
            DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 8.0, 8.0),
                color: Color::BLACK,
            },
        );
        previous.push(DisplayItemId::test(3), DisplayCommand::PopClip);

        let mut current = previous.clone();
        current.commands[1] = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 8.0, 8.0),
            color: Color::rgb(0, 255, 0),
        };
        let damage = DamageRegion::between(Some(&previous), &current);

        let mut incremental = Framebuffer::new(
            Size {
                width: 8.0,
                height: 8.0,
            },
            Color::WHITE,
        );
        incremental.rasterize(&previous);
        incremental.rasterize_damage(&current, &damage, Color::WHITE);

        let mut full = Framebuffer::new(
            Size {
                width: 8.0,
                height: 8.0,
            },
            Color::WHITE,
        );
        full.rasterize(&current);
        assert_eq!(incremental.pixels, full.pixels);
    }
}
