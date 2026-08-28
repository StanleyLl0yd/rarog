from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker: {label}")
    return text.replace(old, new, 1)


path = Path("crates/rarog-paint/src/lib.rs")
text = path.read_text()
text = replace_once(
    text,
    "use rarog_types::{Color, Rect, Size};",
    "use rarog_types::{Color, Point, Rect, Size};",
    "paint imports",
)

marker = """#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StackingContextId(pub u64);

"""
addition = marker + """#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub m11: f32,
    pub m12: f32,
    pub m21: f32,
    pub m22: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub const fn translation(tx: f32, ty: f32) -> Self {
        Self { tx, ty, ..Self::IDENTITY }
    }

    pub const fn scale(x: f32, y: f32) -> Self {
        Self {
            m11: x,
            m22: y,
            ..Self::IDENTITY
        }
    }

    pub fn is_finite(self) -> bool {
        [self.m11, self.m12, self.m21, self.m22, self.tx, self.ty]
            .into_iter()
            .all(f32::is_finite)
    }

    fn transform_point(self, point: Point) -> Point {
        Point {
            x: self.m11 * point.x + self.m21 * point.y + self.tx,
            y: self.m12 * point.x + self.m22 * point.y + self.ty,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opacity(f32);

impl Opacity {
    pub const ONE: Self = Self(1.0);

    pub fn new(value: f32) -> Option<Self> {
        value.is_finite().then(|| Self(value.clamp(0.0, 1.0)))
    }

    pub const fn value(self) -> f32 {
        self.0
    }

    fn multiply(self, other: Self) -> Self {
        Self((self.0 * other.0).clamp(0.0, 1.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructuralScope {
    Clip,
    Stacking,
    Transform,
    Opacity,
}

"""
text = replace_once(text, marker, addition, "stacking marker")

old = """#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
    PushClip { rect: Rect },
    PopClip,
    PushStackingContext { id: StackingContextId },
    PopStackingContext,
}
"""
new = """#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
    PushClip { rect: Rect },
    PopClip,
    PushStackingContext { id: StackingContextId },
    PopStackingContext,
    PushTransform { transform: Transform2D },
    PopTransform,
    PushOpacity { opacity: Opacity },
    PopOpacity,
}
"""
text = replace_once(text, old, new, "display command enum")

old = """    pub fn bounds(self) -> Option<Rect> {
        match self {
            Self::FillRect { rect, .. }
            | Self::TextPlaceholder { rect, .. }
            | Self::PushClip { rect } => Some(rect),
            Self::PopClip | Self::PushStackingContext { .. } | Self::PopStackingContext => None,
        }
    }

    fn is_structural(self) -> bool {
        matches!(
            self,
            Self::PushClip { .. }
                | Self::PopClip
                | Self::PushStackingContext { .. }
                | Self::PopStackingContext
        )
    }
"""
new = """    pub fn bounds(self) -> Option<Rect> {
        match self {
            Self::FillRect { rect, .. }
            | Self::TextPlaceholder { rect, .. }
            | Self::PushClip { rect } => Some(rect),
            Self::PopClip
            | Self::PushStackingContext { .. }
            | Self::PopStackingContext
            | Self::PushTransform { .. }
            | Self::PopTransform
            | Self::PushOpacity { .. }
            | Self::PopOpacity => None,
        }
    }

    fn is_structural(self) -> bool {
        matches!(
            self,
            Self::PushClip { .. }
                | Self::PopClip
                | Self::PushStackingContext { .. }
                | Self::PopStackingContext
                | Self::PushTransform { .. }
                | Self::PopTransform
                | Self::PushOpacity { .. }
                | Self::PopOpacity
        )
    }
"""
text = replace_once(text, old, new, "command bounds")

start = text.index("    pub fn has_balanced_structure(&self) -> bool {")
end = text.index("\n    pub fn snapshot(&self) -> String {", start)
replacement = """    pub fn has_balanced_structure(&self) -> bool {
        let mut scopes = Vec::new();
        for command in &self.commands {
            if !apply_scope_command(*command, &mut scopes) {
                return false;
            }
        }
        scopes.is_empty()
    }
"""
text = text[:start] + replacement + text[end:]

old = """                DisplayCommand::PopStackingContext => output.push_str(&format!(
                    "{}|pop-stacking-context\\n",
                    display_item_id_snapshot(*id)
                )),
"""
new = old + """                DisplayCommand::PushTransform { transform } => output.push_str(&format!(
                    "{}|push-transform|{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}\\n",
                    display_item_id_snapshot(*id),
                    transform.m11,
                    transform.m12,
                    transform.m21,
                    transform.m22,
                    transform.tx,
                    transform.ty
                )),
                DisplayCommand::PopTransform => output.push_str(&format!(
                    "{}|pop-transform\\n",
                    display_item_id_snapshot(*id)
                )),
                DisplayCommand::PushOpacity { opacity } => output.push_str(&format!(
                    "{}|push-opacity|{:.4}\\n",
                    display_item_id_snapshot(*id),
                    opacity.value()
                )),
                DisplayCommand::PopOpacity => output.push_str(&format!(
                    "{}|pop-opacity\\n",
                    display_item_id_snapshot(*id)
                )),
"""
text = replace_once(text, old, new, "snapshot scopes")

old = """    if previous.command_ids.is_empty() {
        return current.command_ids.is_empty();
    }
    if !previous.has_balanced_structure() || !current.has_balanced_structure() {
        return false;
    }
"""
new = """    if previous.command_ids.is_empty() {
        return current.command_ids.is_empty();
    }
    if !list.has_balanced_structure()
        || !previous.has_balanced_structure()
        || !current.has_balanced_structure()
    {
        return false;
    }
"""
text = replace_once(text, old, new, "retained initial validation")

old = """    if range.len() != previous.command_ids.len() {
        return false;
    }

    let mut candidate = list.clone();
"""
new = """    if range.len() != previous.command_ids.len() {
        return false;
    }
    if list.commands[range.start..range.end] != previous.commands[..] {
        return false;
    }
    let (Some(before_scopes), Some(after_scopes)) = (
        scope_stack_at(&list.commands, range.start),
        scope_stack_at(&list.commands, range.end),
    ) else {
        return false;
    };
    if before_scopes != after_scopes {
        return false;
    }

    let mut candidate = list.clone();
"""
text = replace_once(text, old, new, "retained scope validation")

marker = "fn collect(fragment: &Fragment, list: &mut DisplayList) {\n"
helpers = """fn apply_scope_command(command: DisplayCommand, scopes: &mut Vec<StructuralScope>) -> bool {
    match command {
        DisplayCommand::PushClip { .. } => scopes.push(StructuralScope::Clip),
        DisplayCommand::PopClip => return scopes.pop() == Some(StructuralScope::Clip),
        DisplayCommand::PushStackingContext { .. } => scopes.push(StructuralScope::Stacking),
        DisplayCommand::PopStackingContext => {
            return scopes.pop() == Some(StructuralScope::Stacking);
        }
        DisplayCommand::PushTransform { transform } => {
            if !transform.is_finite() {
                return false;
            }
            scopes.push(StructuralScope::Transform);
        }
        DisplayCommand::PopTransform => return scopes.pop() == Some(StructuralScope::Transform),
        DisplayCommand::PushOpacity { .. } => scopes.push(StructuralScope::Opacity),
        DisplayCommand::PopOpacity => return scopes.pop() == Some(StructuralScope::Opacity),
        DisplayCommand::FillRect { .. } | DisplayCommand::TextPlaceholder { .. } => {}
    }
    true
}

fn scope_stack_at(commands: &[DisplayCommand], boundary: usize) -> Option<Vec<StructuralScope>> {
    if boundary > commands.len() {
        return None;
    }
    let mut scopes = Vec::new();
    for command in &commands[..boundary] {
        if !apply_scope_command(*command, &mut scopes) {
            return None;
        }
    }
    Some(scopes)
}

"""
text = replace_once(text, marker, helpers + marker, "collect marker")

old = """        let Some(previous) = previous else {
            let mut damage = Self::default();
            for command in &current.commands {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            return damage;
        };
"""
new = """        let Some(previous) = previous else {
            let mut damage = Self::default();
            for bounds in effective_paint_bounds(current) {
                damage.push(bounds);
            }
            return damage;
        };
"""
text = replace_once(text, old, new, "initial damage")

old = """        if previous
            .commands
            .iter()
            .copied()
            .any(DisplayCommand::is_structural)
            || current
                .commands
                .iter()
                .copied()
                .any(DisplayCommand::is_structural)
        {
            let mut damage = Self::default();
            for command in previous.commands.iter().chain(&current.commands) {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            return damage;
        }
"""
new = """        if previous
            .commands
            .iter()
            .copied()
            .any(DisplayCommand::is_structural)
            || current
                .commands
                .iter()
                .copied()
                .any(DisplayCommand::is_structural)
        {
            let mut damage = Self::default();
            for bounds in effective_paint_bounds(previous)
                .into_iter()
                .chain(effective_paint_bounds(current))
            {
                damage.push(bounds);
            }
            return damage;
        }
"""
text = replace_once(text, old, new, "structural damage")

marker = "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum FramebufferError {"
helpers = """fn transform_rect(rect: Rect, transforms: &[Transform2D]) -> Rect {
    let corners = [
        rect.origin,
        Point {
            x: rect.origin.x + rect.size.width,
            y: rect.origin.y,
        },
        Point {
            x: rect.origin.x,
            y: rect.origin.y + rect.size.height,
        },
        Point {
            x: rect.origin.x + rect.size.width,
            y: rect.origin.y + rect.size.height,
        },
    ];
    let transformed = corners.map(|mut point| {
        for transform in transforms {
            point = transform.transform_point(point);
        }
        point
    });
    let min_x = transformed
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let min_y = transformed
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_x = transformed
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = transformed
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);
    Rect::new(
        min_x,
        min_y,
        (max_x - min_x).max(0.0),
        (max_y - min_y).max(0.0),
    )
}

fn effective_paint_bounds(list: &DisplayList) -> Vec<Rect> {
    let mut output = Vec::new();
    let mut transforms = Vec::new();
    let mut clips: Vec<Option<Rect>> = vec![None];
    for command in &list.commands {
        match *command {
            DisplayCommand::FillRect { rect, .. }
            | DisplayCommand::TextPlaceholder { rect, .. } => {
                let rect = transform_rect(rect, &transforms);
                let bounds = match *clips.last().expect("clip state") {
                    Some(clip) => intersection(rect, clip),
                    None => Some(rect),
                };
                if let Some(bounds) = bounds {
                    output.push(bounds);
                }
            }
            DisplayCommand::PushClip { rect } => {
                let rect = transform_rect(rect, &transforms);
                let clip = match *clips.last().expect("clip state") {
                    Some(current) => intersection(current, rect)
                        .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
                    None => rect,
                };
                clips.push(Some(clip));
            }
            DisplayCommand::PopClip => {
                clips.pop();
            }
            DisplayCommand::PushTransform { transform } => transforms.push(transform),
            DisplayCommand::PopTransform => {
                transforms.pop();
            }
            DisplayCommand::PushStackingContext { .. }
            | DisplayCommand::PopStackingContext
            | DisplayCommand::PushOpacity { .. }
            | DisplayCommand::PopOpacity => {}
        }
    }
    output
}

"""
text = replace_once(text, marker, helpers + marker, "framebuffer error marker")

start = text.index("    pub fn rasterize(&mut self, list: &DisplayList) {")
end = text.index("\n    pub fn rasterize_damage(", start)
replacement = """    pub fn rasterize(&mut self, list: &DisplayList) {
        assert!(
            list.has_balanced_structure(),
            "display list structural scopes must be balanced"
        );
        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);
        let mut clips = vec![framebuffer_clip];
        let mut transforms = Vec::new();
        let mut opacities = vec![Opacity::ONE];
        for command in &list.commands {
            match *command {
                DisplayCommand::FillRect { rect, color }
                | DisplayCommand::TextPlaceholder { rect, color } => {
                    let rect = transform_rect(rect, &transforms);
                    if let Some(clipped) = intersection(rect, *clips.last().expect("clip stack")) {
                        let color = apply_opacity(color, *opacities.last().expect("opacity stack"));
                        self.fill_rect(clipped, color);
                    }
                }
                DisplayCommand::PushClip { rect } => {
                    let rect = transform_rect(rect, &transforms);
                    let current = *clips.last().expect("clip stack");
                    clips.push(
                        intersection(current, rect)
                            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
                    );
                }
                DisplayCommand::PopClip => {
                    clips.pop();
                }
                DisplayCommand::PushStackingContext { .. }
                | DisplayCommand::PopStackingContext => {}
                DisplayCommand::PushTransform { transform } => transforms.push(transform),
                DisplayCommand::PopTransform => {
                    transforms.pop();
                }
                DisplayCommand::PushOpacity { opacity } => {
                    let current = *opacities.last().expect("opacity stack");
                    opacities.push(current.multiply(opacity));
                }
                DisplayCommand::PopOpacity => {
                    opacities.pop();
                }
            }
        }
    }
"""
text = text[:start] + replacement + text[end:]

old = """                    DisplayCommand::PushClip { .. }
                    | DisplayCommand::PopClip
                    | DisplayCommand::PushStackingContext { .. }
                    | DisplayCommand::PopStackingContext => continue,
"""
new = """                    DisplayCommand::PushClip { .. }
                    | DisplayCommand::PopClip
                    | DisplayCommand::PushStackingContext { .. }
                    | DisplayCommand::PopStackingContext
                    | DisplayCommand::PushTransform { .. }
                    | DisplayCommand::PopTransform
                    | DisplayCommand::PushOpacity { .. }
                    | DisplayCommand::PopOpacity => continue,
"""
text = replace_once(text, old, new, "damage structural match")

old = """        for y in y0..y1 {
            for x in x0..x1 {
                self.pixels[(y * self.width + x) as usize] = color;
            }
        }
"""
new = """        for y in y0..y1 {
            for x in x0..x1 {
                let index = (y * self.width + x) as usize;
                self.pixels[index] = blend_over(self.pixels[index], color);
            }
        }
"""
text = replace_once(text, old, new, "fill rect blend")

marker = "fn intersection(a: Rect, b: Rect) -> Option<Rect> {\n"
helpers = """fn apply_opacity(color: Color, opacity: Opacity) -> Color {
    let alpha = (f32::from(color.a) * opacity.value())
        .round()
        .clamp(0.0, 255.0) as u8;
    Color { a: alpha, ..color }
}

fn blend_over(destination: Color, source: Color) -> Color {
    if source.a == 0 {
        return destination;
    }
    if source.a == u8::MAX {
        return source;
    }
    let source_alpha = u32::from(source.a);
    let destination_alpha = u32::from(destination.a);
    let inverse = 255 - source_alpha;
    let output_alpha = source_alpha + (destination_alpha * inverse + 127) / 255;
    if output_alpha == 0 {
        return Color::TRANSPARENT;
    }
    let channel = |source_channel: u8, destination_channel: u8| -> u8 {
        let source_premultiplied = u32::from(source_channel) * source_alpha;
        let destination_premultiplied =
            (u32::from(destination_channel) * destination_alpha * inverse + 127) / 255;
        ((source_premultiplied + destination_premultiplied + output_alpha / 2) / output_alpha)
            .min(255) as u8
    };
    Color {
        r: channel(source.r, destination.r),
        g: channel(source.g, destination.g),
        b: channel(source.b, destination.b),
        a: output_alpha.min(255) as u8,
    }
}

"""
text = replace_once(text, marker, helpers + marker, "intersection marker")

marker = """    #[test]
    fn damage_tracks_changed_display_items() {
"""
tests = """    #[test]
    fn transform_and_opacity_scopes_are_balanced_and_rasterized() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(7),
            },
        );
        list.push(
            DisplayItemId::test(2),
            DisplayCommand::PushClip {
                rect: Rect::new(0.0, 0.0, 6.0, 4.0),
            },
        );
        list.push(
            DisplayItemId::test(3),
            DisplayCommand::PushTransform {
                transform: Transform2D::translation(2.0, 1.0),
            },
        );
        list.push(
            DisplayItemId::test(4),
            DisplayCommand::PushOpacity {
                opacity: Opacity::new(0.5).unwrap(),
            },
        );
        list.push(
            DisplayItemId::test(5),
            DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                color: Color::BLACK,
            },
        );
        list.push(DisplayItemId::test(6), DisplayCommand::PopOpacity);
        list.push(DisplayItemId::test(7), DisplayCommand::PopTransform);
        list.push(DisplayItemId::test(8), DisplayCommand::PopClip);
        list.push(DisplayItemId::test(9), DisplayCommand::PopStackingContext);

        assert!(list.has_balanced_structure());
        assert!(list.snapshot().contains("push-transform"));
        assert!(list.snapshot().contains("push-opacity|0.5000"));

        let mut framebuffer = Framebuffer::new(
            Size {
                width: 6.0,
                height: 4.0,
            },
            Color::WHITE,
        );
        framebuffer.rasterize(&list);
        assert_eq!(
            framebuffer.pixels[(1 * 6 + 2) as usize],
            Color::rgb(127, 127, 127)
        );
        assert_eq!(framebuffer.pixels[0], Color::WHITE);
    }

    #[test]
    fn non_finite_transform_is_structurally_invalid() {
        let list = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(2)],
            commands: vec![
                DisplayCommand::PushTransform {
                    transform: Transform2D::translation(f32::NAN, 0.0),
                },
                DisplayCommand::PopTransform,
            ],
        };
        assert!(!list.has_balanced_structure());
    }

    #[test]
    fn structural_damage_uses_transformed_paint_bounds() {
        let make = |translation: f32| DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
            ],
            commands: vec![
                DisplayCommand::PushTransform {
                    transform: Transform2D::translation(translation, 0.0),
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                    color: Color::BLACK,
                },
                DisplayCommand::PopTransform,
            ],
        };
        let before = make(0.0);
        let after = make(10.0);
        let damage = DamageRegion::between(Some(&before), &after);
        assert!(damage.rects.contains(&Rect::new(0.0, 0.0, 2.0, 2.0)));
        assert!(damage.rects.contains(&Rect::new(10.0, 0.0, 2.0, 2.0)));
    }

    #[test]
    fn retained_patch_requires_exact_previous_commands() {
        let mut list = single_fill(1, Rect::new(0.0, 0.0, 2.0, 2.0), Color::BLACK);
        let previous = single_fill(1, Rect::new(1.0, 0.0, 2.0, 2.0), Color::BLACK);
        let current = single_fill(1, Rect::new(2.0, 0.0, 2.0, 2.0), Color::BLACK);
        assert!(!replace_display_items(&mut list, &previous, &current));
    }

    #[test]
    fn retained_fragment_patch_is_safe_inside_stacking_and_clip_scopes() {
        let stacking = DisplayItemId {
            source: 9,
            fragment: 0,
            slot: 10,
        };
        let clip = DisplayItemId {
            source: 9,
            fragment: 0,
            slot: 11,
        };
        let first = DisplayItemId {
            source: 42,
            fragment: 0,
            slot: 0,
        };
        let second = DisplayItemId {
            source: 42,
            fragment: 1,
            slot: 0,
        };
        let pop_clip = DisplayItemId {
            source: 9,
            fragment: 0,
            slot: 12,
        };
        let pop_stacking = DisplayItemId {
            source: 9,
            fragment: 0,
            slot: 13,
        };
        let first_command = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 2.0, 2.0),
            color: Color::BLACK,
        };
        let second_command = DisplayCommand::FillRect {
            rect: Rect::new(2.0, 0.0, 2.0, 2.0),
            color: Color::BLACK,
        };
        let mut list = DisplayList {
            command_ids: vec![stacking, clip, first, second, pop_clip, pop_stacking],
            commands: vec![
                DisplayCommand::PushStackingContext {
                    id: StackingContextId(9),
                },
                DisplayCommand::PushClip {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                },
                first_command,
                second_command,
                DisplayCommand::PopClip,
                DisplayCommand::PopStackingContext,
            ],
        };
        let previous = DisplayList {
            command_ids: vec![second],
            commands: vec![second_command],
        };
        let replacement = DisplayCommand::FillRect {
            rect: Rect::new(3.0, 0.0, 2.0, 2.0),
            color: Color::BLACK,
        };
        let current = DisplayList {
            command_ids: vec![second],
            commands: vec![replacement],
        };

        assert!(replace_display_items(&mut list, &previous, &current));
        assert_eq!(list.commands[2], first_command);
        assert_eq!(list.commands[3], replacement);
        assert!(list.has_balanced_structure());
    }

"""
text = replace_once(text, marker, tests + marker, "test insertion marker")
path.write_text(text)

backlog = Path("docs/R0-BACKLOG.md")
text = backlog.read_text()
text = text.replace(
    "- [ ] transforms/opacity representation",
    "- [x] transforms/opacity representation",
)
text = text.replace(
    "- [ ] fragmentation/stacking/clip-aware retained display-list updates",
    "- [x] fragmentation/stacking/clip-aware retained display-list updates",
)
backlog.write_text(text)

architecture = Path("docs/ARCHITECTURE.md")
text = architecture.read_text()
old = """Stacking contexts are represented as explicit balanced display-list scopes with stable context IDs. R0 does not yet assign CSS stacking order, opacity, transforms, or compositing behavior to these scopes; the representation exists so those semantics can be added without changing the display-list contract again.

Retained display-list replacement operates on exact contiguous command ranges rather than unordered ID sets. A patch is accepted only when the previous range is contiguous and both replacement and resulting lists preserve unique IDs and balanced structural scopes.
"""
new = """Stacking contexts, transforms and opacity are represented as explicit balanced display-list scopes. `Transform2D` is a backend-neutral affine transform and `Opacity` is a clamped scalar. The R0 software raster path applies nested transforms to rectangular paint bounds, intersects transformed clips in device space and source-over blends opacity-modulated colors. This remains a bootstrap raster model: it does not define CSS transform-origin, stacking order, isolation groups or compositor surfaces.

Retained display-list replacement operates on exact contiguous command ranges rather than unordered ID sets. A patch is accepted only when the live range still contains the exact previous commands, the range begins and ends in the same outer structural scope state, and the replacement/result preserve unique IDs and balanced clip/stacking/transform/opacity scopes. Because display-item identity includes fragment ordinal, one fragment can be patched inside nested stacking/clip scopes without colliding with sibling fragments from the same source node.
"""
text = replace_once(text, old, new, "architecture retained paint")
old = """The current `DamageRegion` intentionally stores conservative rectangles without advanced coalescing. R0 can replace the stable command range belonging to an affected fragment subtree and preserve unrelated commands; if a stable previous range does not exist, it falls back to display-list regeneration. The software framebuffer clears damaged rectangles to the frame background and replays only command portions intersecting those rectangles. Clip/transform-aware damage, occlusion, stacking-aware retained updates and compositor damage remain later work.
"""
new = """The current `DamageRegion` intentionally stores conservative rectangles without advanced coalescing. For structural display lists it derives conservative device-space paint bounds through transform and clip scopes; structural damage rasterization still uses a full-frame refresh so correctness does not depend on partial replay across compositing scopes. R0 can replace the stable command range belonging to an affected fragment subtree and preserve unrelated commands; if a stable previous range or structural proof does not exist, it falls back to display-list regeneration. Occlusion, CSS stacking-order semantics, isolated opacity groups and compositor damage remain later work.
"""
text = replace_once(text, old, new, "architecture damage")
architecture.write_text(text)

Path("docs/adr/ADR-0027-transform-opacity-and-retained-paint-scopes.md").write_text(
    """# ADR-0027: Transform, opacity and retained paint scopes

## Status

Accepted.

## Context

R0 already has explicit clip and stacking-context commands plus contiguous retained display-list replacement. The remaining paint boundary needs to represent transforms and opacity without coupling paint to a particular compositor, and retained replacement needs a stronger proof when fragment ranges live inside nested structural scopes.

## Decision

The display list gains balanced transform and opacity scopes alongside clip and stacking scopes. `Transform2D` carries a backend-neutral affine transform. `Opacity` is finite and clamped to `[0, 1]`. The bootstrap software rasterizer applies nested transforms to rectangular paint geometry, evaluates clips in transformed device space, multiplies nested opacity and uses deterministic source-over blending. This is a representation and correctness foundation, not CSS transforms/compositing conformance.

Retained replacement now requires more than matching IDs. The live contiguous range must still contain the exact previous commands, and the structural scope stack at the beginning and end of that range must be identical. The replacement and resulting display lists must remain balanced and retain globally unique IDs. Fragment ordinals remain part of display-item identity, so separate fragments from one source node can be patched independently.

Structural damage computes conservative effective paint bounds through transform and clip scopes. Damage-scoped rasterization deliberately keeps the conservative full-frame fallback whenever any structural scope is present; partial replay through stacking/transform/opacity scopes is deferred to compositor work.

## Consequences

- Transform and opacity no longer require another display-list contract redesign.
- Retained patches fail closed if the previous slice is stale or crosses a structural-scope boundary.
- Fragmentation, stacking and clipping can coexist with retained range replacement in the R0 proof model.
- The software path remains deterministic while later GPU/compositor backends may consume the same commands differently.
- CSS transform-origin, 3D transforms, stacking-order calculation, isolated opacity groups, filters, occlusion and partial structural damage replay remain future work.
"""
)
