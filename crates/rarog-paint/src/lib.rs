use rarog_layout::{Fragment, FragmentKind, FragmentTree};
use rarog_types::{Color, Point, Rect, Size};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StackingContextId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
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
        Self {
            tx,
            ty,
            ..Self::IDENTITY
        }
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

impl DisplayCommand {
    pub fn bounds(self) -> Option<Rect> {
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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    command_ids: Vec<DisplayItemId>,
    commands: Vec<DisplayCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayListError {
    LengthMismatch { ids: usize, commands: usize },
    DuplicateIds,
    UnbalancedStructure,
}

impl fmt::Display for DisplayListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { ids, commands } => {
                write!(
                    formatter,
                    "display list has {ids} IDs for {commands} commands"
                )
            }
            Self::DuplicateIds => formatter.write_str("display list contains duplicate item IDs"),
            Self::UnbalancedStructure => {
                formatter.write_str("display list structural scopes are invalid")
            }
        }
    }
}

impl std::error::Error for DisplayListError {}

impl DisplayList {
    pub fn try_from_parts(
        command_ids: Vec<DisplayItemId>,
        commands: Vec<DisplayCommand>,
    ) -> Result<Self, DisplayListError> {
        let list = Self {
            command_ids,
            commands,
        };
        list.validate()?;
        Ok(list)
    }

    pub fn command_ids(&self) -> &[DisplayItemId] {
        &self.command_ids
    }

    pub fn commands(&self) -> &[DisplayCommand] {
        &self.commands
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn validate(&self) -> Result<(), DisplayListError> {
        if self.command_ids.len() != self.commands.len() {
            return Err(DisplayListError::LengthMismatch {
                ids: self.command_ids.len(),
                commands: self.commands.len(),
            });
        }
        if !self.has_unique_ids() {
            return Err(DisplayListError::DuplicateIds);
        }
        if !self.has_balanced_structure() {
            return Err(DisplayListError::UnbalancedStructure);
        }
        Ok(())
    }

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

    pub fn has_balanced_structure(&self) -> bool {
        let mut scopes = Vec::new();
        for command in &self.commands {
            if !apply_scope_command(*command, &mut scopes) {
                return false;
            }
        }
        scopes.is_empty()
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
                DisplayCommand::PushStackingContext { id: context } => output.push_str(&format!(
                    "{}|push-stacking-context|{}\n",
                    display_item_id_snapshot(*id),
                    context.0
                )),
                DisplayCommand::PopStackingContext => output.push_str(&format!(
                    "{}|pop-stacking-context\n",
                    display_item_id_snapshot(*id)
                )),
                DisplayCommand::PushTransform { transform } => output.push_str(&format!(
                    "{}|push-transform|{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}\n",
                    display_item_id_snapshot(*id),
                    transform.m11,
                    transform.m12,
                    transform.m21,
                    transform.m22,
                    transform.tx,
                    transform.ty
                )),
                DisplayCommand::PopTransform => output.push_str(&format!(
                    "{}|pop-transform\n",
                    display_item_id_snapshot(*id)
                )),
                DisplayCommand::PushOpacity { opacity } => output.push_str(&format!(
                    "{}|push-opacity|{:.4}\n",
                    display_item_id_snapshot(*id),
                    opacity.value()
                )),
                DisplayCommand::PopOpacity => {
                    output.push_str(&format!("{}|pop-opacity\n", display_item_id_snapshot(*id)))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayRange {
    pub start: usize,
    pub end: usize,
}

impl DisplayRange {
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

impl DisplayList {
    pub fn contiguous_range_for_ids(&self, ids: &[DisplayItemId]) -> Option<DisplayRange> {
        if ids.is_empty() || ids.len() > self.command_ids.len() {
            return None;
        }
        let start = self
            .command_ids
            .windows(ids.len())
            .position(|window| window == ids)?;
        Some(DisplayRange {
            start,
            end: start + ids.len(),
        })
    }
}

fn replace_display_items(
    list: &mut DisplayList,
    previous: &DisplayList,
    current: &DisplayList,
) -> bool {
    if previous.command_ids.is_empty() {
        return current.command_ids.is_empty();
    }
    if !list.has_balanced_structure()
        || !previous.has_balanced_structure()
        || !current.has_balanced_structure()
    {
        return false;
    }

    let Some(range) = list.contiguous_range_for_ids(&previous.command_ids) else {
        return false;
    };
    if range.len() != previous.command_ids.len() {
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
    candidate
        .command_ids
        .splice(range.start..range.end, current.command_ids.iter().copied());
    candidate
        .commands
        .splice(range.start..range.end, current.commands.iter().copied());

    if !candidate.has_unique_ids() || !candidate.has_balanced_structure() {
        return false;
    }
    *list = candidate;
    true
}

fn apply_scope_command(command: DisplayCommand, scopes: &mut Vec<StructuralScope>) -> bool {
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
        .map(|node| node.index() as u64)
        .unwrap_or_else(|| (1_u64 << 63) | fragment.layout_node.index() as u64);
    DisplayItemId {
        source,
        fragment: u64::from(fragment.ordinal.index()),
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
        let Some(previous) = previous else {
            let mut damage = Self::default();
            for bounds in effective_paint_bounds(current) {
                damage.push(bounds);
            }
            return damage;
        };

        if previous
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

fn transform_rect(rect: Rect, transforms: &[Transform2D]) -> Rect {
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
                    Some(current) => {
                        intersection(current, rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
                    }
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
    #[cfg(test)]
    fn new(size: Size, background: Color) -> Self {
        Self::try_new(size, background).expect("test framebuffer dimensions are valid")
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
                    clips
                        .push(intersection(current, rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)));
                }
                DisplayCommand::PopClip => {
                    clips.pop();
                }
                DisplayCommand::PushStackingContext { .. } | DisplayCommand::PopStackingContext => {
                }
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

    pub fn rasterize_damage(
        &mut self,
        list: &DisplayList,
        damage: &DamageRegion,
        background: Color,
    ) {
        if list
            .commands
            .iter()
            .copied()
            .any(DisplayCommand::is_structural)
        {
            self.clear_rect(
                Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
                background,
            );
            self.rasterize(list);
            return;
        }
        for damaged in &damage.rects {
            self.clear_rect(*damaged, background);
            for command in &list.commands {
                let (rect, color) = match *command {
                    DisplayCommand::FillRect { rect, color }
                    | DisplayCommand::TextPlaceholder { rect, color } => (rect, color),
                    DisplayCommand::PushClip { .. }
                    | DisplayCommand::PopClip
                    | DisplayCommand::PushStackingContext { .. }
                    | DisplayCommand::PopStackingContext
                    | DisplayCommand::PushTransform { .. }
                    | DisplayCommand::PopTransform
                    | DisplayCommand::PushOpacity { .. }
                    | DisplayCommand::PopOpacity => continue,
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
                let index = (y * self.width + x) as usize;
                self.pixels[index] = blend_over(self.pixels[index], color);
            }
        }
    }

    fn clear_rect(&mut self, rect: Rect, color: Color) {
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

fn apply_opacity(color: Color, opacity: Opacity) -> Color {
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
        assert_eq!(framebuffer.pixels[8], Color::rgb(127, 127, 127));
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

    #[test]
    fn damage_clear_overwrites_with_transparent_background() {
        let size = Size {
            width: 2.0,
            height: 2.0,
        };
        let mut framebuffer = Framebuffer::new(size, Color::BLACK);
        let damage = DamageRegion {
            rects: vec![Rect::new(0.0, 0.0, 1.0, 1.0)],
        };
        framebuffer.rasterize_damage(&DisplayList::default(), &damage, Color::TRANSPARENT);

        assert_eq!(framebuffer.pixels[0], Color::TRANSPARENT);
        assert_eq!(framebuffer.pixels[1], Color::BLACK);
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
    fn retained_range_requires_exact_contiguous_ids() {
        let list = DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
            ],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(2.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        assert_eq!(
            list.contiguous_range_for_ids(&[DisplayItemId::test(2), DisplayItemId::test(3)]),
            Some(DisplayRange { start: 1, end: 3 })
        );
        assert_eq!(
            list.contiguous_range_for_ids(&[DisplayItemId::test(1), DisplayItemId::test(3)]),
            None
        );
    }

    #[test]
    fn retained_patch_rejects_noncontiguous_previous_items() {
        let mut list = DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
            ],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(2.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        let previous = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(3)],
            commands: vec![list.commands[0], list.commands[2]],
        };
        let current = previous.clone();
        assert!(!replace_display_items(&mut list, &previous, &current));
    }

    #[test]
    fn retained_patch_failure_is_atomic() {
        let mut list = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(2)],
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
        let original = list.clone();
        let previous = DisplayList {
            command_ids: vec![DisplayItemId::test(1)],
            commands: vec![list.commands[0]],
        };
        let current = DisplayList {
            command_ids: vec![DisplayItemId::test(2)],
            commands: vec![DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 2.0, 1.0),
                color: Color::BLACK,
            }],
        };

        assert!(!replace_display_items(&mut list, &previous, &current));
        assert_eq!(list, original);
    }

    #[test]
    fn retained_patch_preserves_balanced_structural_scopes() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(1),
            },
        );
        list.push(
            DisplayItemId::test(2),
            DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                color: Color::BLACK,
            },
        );
        list.push(DisplayItemId::test(3), DisplayCommand::PopStackingContext);

        let previous = list.clone();
        let mut current = previous.clone();
        current.commands[1] = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 3.0, 2.0),
            color: Color::BLACK,
        };
        assert!(replace_display_items(&mut list, &previous, &current));
        assert!(list.has_balanced_structure());
        assert_eq!(list, current);
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
    fn nested_transform_order_follows_display_list_push_order() {
        let rect = Rect::new(1.0, 1.0, 2.0, 2.0);
        let transformed = transform_rect(
            rect,
            &[
                Transform2D::translation(10.0, 0.0),
                Transform2D::scale(2.0, 3.0),
            ],
        );
        assert_eq!(transformed, Rect::new(22.0, 3.0, 4.0, 6.0));
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
    fn malformed_external_display_lists_are_rejected_without_panicking() {
        let id = DisplayItemId {
            source: 1,
            fragment: 2,
            slot: 0,
        };
        let command = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            color: Color::BLACK,
        };

        assert_eq!(
            DisplayList::try_from_parts(vec![id], vec![command, command]),
            Err(DisplayListError::LengthMismatch {
                ids: 1,
                commands: 2
            })
        );
        assert_eq!(
            DisplayList::try_from_parts(vec![id, id], vec![command, command]),
            Err(DisplayListError::DuplicateIds)
        );
        assert_eq!(
            DisplayList::try_from_parts(vec![id], vec![DisplayCommand::PopClip],),
            Err(DisplayListError::UnbalancedStructure)
        );
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

    #[test]
    fn stacking_context_commands_have_deterministic_snapshots() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(20),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(42),
            },
        );
        list.push(DisplayItemId::test(21), DisplayCommand::PopStackingContext);

        assert_eq!(
            list.snapshot(),
            "20:20:0|push-stacking-context|42\n21:21:0|pop-stacking-context\n"
        );
        assert!(list.has_balanced_structure());
    }

    #[test]
    fn structural_scopes_must_be_properly_nested() {
        let mut valid = DisplayList::default();
        valid.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(1),
            },
        );
        valid.push(
            DisplayItemId::test(2),
            DisplayCommand::PushClip {
                rect: Rect::new(0.0, 0.0, 4.0, 4.0),
            },
        );
        valid.push(DisplayItemId::test(3), DisplayCommand::PopClip);
        valid.push(DisplayItemId::test(4), DisplayCommand::PopStackingContext);
        assert!(valid.has_balanced_structure());

        let mut invalid = valid.clone();
        invalid.commands.swap(2, 3);
        assert!(!invalid.has_balanced_structure());
    }

    #[test]
    fn stacking_context_boundaries_preserve_current_raster_output() {
        let rect = Rect::new(1.0, 1.0, 3.0, 3.0);
        let mut plain = DisplayList::default();
        plain.push(
            DisplayItemId::test(1),
            DisplayCommand::FillRect {
                rect,
                color: Color::BLACK,
            },
        );

        let mut stacked = DisplayList::default();
        stacked.push(
            DisplayItemId::test(2),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(9),
            },
        );
        stacked.push(
            DisplayItemId::test(3),
            DisplayCommand::FillRect {
                rect,
                color: Color::BLACK,
            },
        );
        stacked.push(DisplayItemId::test(4), DisplayCommand::PopStackingContext);

        let size = Size {
            width: 6.0,
            height: 6.0,
        };
        let mut plain_fb = Framebuffer::new(size, Color::WHITE);
        plain_fb.rasterize(&plain);
        let mut stacked_fb = Framebuffer::new(size, Color::WHITE);
        stacked_fb.rasterize(&stacked);
        assert_eq!(plain_fb.pixels, stacked_fb.pixels);
    }

    #[test]
    fn damage_raster_with_stacking_contexts_matches_full_raster() {
        let mut previous = DisplayList::default();
        previous.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(1),
            },
        );
        previous.push(
            DisplayItemId::test(2),
            DisplayCommand::FillRect {
                rect: Rect::new(1.0, 1.0, 4.0, 4.0),
                color: Color::BLACK,
            },
        );
        previous.push(DisplayItemId::test(3), DisplayCommand::PopStackingContext);

        let mut current = previous.clone();
        current.commands[1] = DisplayCommand::FillRect {
            rect: Rect::new(1.0, 1.0, 4.0, 4.0),
            color: Color::rgb(0, 0, 255),
        };
        let damage = DamageRegion::between(Some(&previous), &current);

        let size = Size {
            width: 6.0,
            height: 6.0,
        };
        let mut incremental = Framebuffer::new(size, Color::WHITE);
        incremental.rasterize(&previous);
        incremental.rasterize_damage(&current, &damage, Color::WHITE);
        let mut full = Framebuffer::new(size, Color::WHITE);
        full.rasterize(&current);
        assert_eq!(incremental.pixels, full.pixels);
    }
}
