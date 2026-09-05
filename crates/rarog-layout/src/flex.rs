use crate::LayoutNodeId;
use rarog_css::EdgeSizes;
use rarog_types::{Point, Rect, Size};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexRowItem {
    pub node: LayoutNodeId,
    pub base_size: Size,
    pub margin: EdgeSizes,
}

impl FlexRowItem {
    pub const fn new(node: LayoutNodeId, base_size: Size, margin: EdgeSizes) -> Self {
        Self {
            node,
            base_size,
            margin,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexibleFlexRowItem {
    pub item: FlexRowItem,
    pub grow: f32,
    pub shrink: f32,
    pub min_main_size: f32,
    pub max_main_size: Option<f32>,
}

impl FlexibleFlexRowItem {
    pub const fn new(item: FlexRowItem, grow: f32, shrink: f32) -> Self {
        Self {
            item,
            grow,
            shrink,
            min_main_size: 0.0,
            max_main_size: None,
        }
    }

    pub const fn with_main_size_limits(
        mut self,
        min_main_size: f32,
        max_main_size: Option<f32>,
    ) -> Self {
        self.min_main_size = min_main_size;
        self.max_main_size = max_main_size;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexMainAlignment {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexCrossAlignment {
    #[default]
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexContentAlignment {
    #[default]
    Stretch,
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexRowOptions {
    main_alignment: FlexMainAlignment,
    cross_alignment: FlexCrossAlignment,
    content_alignment: FlexContentAlignment,
    main_gap: f32,
    cross_gap: f32,
    cross_size: Option<f32>,
    min_cross_size: Option<f32>,
    max_cross_size: Option<f32>,
}

impl Default for FlexRowOptions {
    fn default() -> Self {
        Self {
            main_alignment: FlexMainAlignment::Start,
            cross_alignment: FlexCrossAlignment::Start,
            content_alignment: FlexContentAlignment::Stretch,
            main_gap: 0.0,
            cross_gap: 0.0,
            cross_size: None,
            min_cross_size: None,
            max_cross_size: None,
        }
    }
}

impl FlexRowOptions {
    pub const fn with_main_alignment(mut self, alignment: FlexMainAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    pub const fn with_cross_alignment(mut self, alignment: FlexCrossAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }

    pub const fn with_content_alignment(mut self, alignment: FlexContentAlignment) -> Self {
        self.content_alignment = alignment;
        self
    }

    pub const fn with_main_gap(mut self, gap: f32) -> Self {
        self.main_gap = gap;
        self
    }

    pub const fn with_cross_gap(mut self, gap: f32) -> Self {
        self.cross_gap = gap;
        self
    }

    pub const fn with_cross_size(mut self, size: Option<f32>) -> Self {
        self.cross_size = size;
        self
    }

    pub const fn with_cross_size_limits(
        mut self,
        minimum: Option<f32>,
        maximum: Option<f32>,
    ) -> Self {
        self.min_cross_size = minimum;
        self.max_cross_size = maximum;
        self
    }

    pub const fn main_alignment(self) -> FlexMainAlignment {
        self.main_alignment
    }

    pub const fn cross_alignment(self) -> FlexCrossAlignment {
        self.cross_alignment
    }

    pub const fn content_alignment(self) -> FlexContentAlignment {
        self.content_alignment
    }

    pub const fn main_gap(self) -> f32 {
        self.main_gap
    }

    pub const fn cross_gap(self) -> f32 {
        self.cross_gap
    }

    pub const fn cross_size(self) -> Option<f32> {
        self.cross_size
    }

    pub const fn min_cross_size(self) -> Option<f32> {
        self.min_cross_size
    }

    pub const fn max_cross_size(self) -> Option<f32> {
        self.max_cross_size
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexRowPlacement {
    pub node: LayoutNodeId,
    pub border_box: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlexRowLayout {
    pub items: Vec<FlexRowPlacement>,
    pub content_size: Size,
    pub overflows_main_axis: bool,
    pub overflows_cross_axis: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlexMultiLineLayout {
    pub items: Vec<FlexRowPlacement>,
    pub content_size: Size,
    pub line_count: usize,
    pub overflows_main_axis: bool,
    pub overflows_cross_axis: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexLayoutError {
    InvalidAvailableSize,
    InvalidOrigin,
    InvalidGap,
    InvalidCrossSize,
    InvalidItemAlignmentCount { expected: usize, actual: usize },
    InvalidItemSize { node: LayoutNodeId },
    InvalidMargin { node: LayoutNodeId },
    NegativeMarginUnsupported { node: LayoutNodeId },
    InvalidFlexFactor { node: LayoutNodeId },
    InvalidFlexMainSizeLimit { node: LayoutNodeId },
    FlexMainSizeLimitRequiresRedistribution { node: LayoutNodeId },
    ShrinkWouldProduceNegativeSize { node: LayoutNodeId },
    GeometryOverflow,
}

impl fmt::Display for FlexLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAvailableSize => {
                formatter.write_str("flex available size must be finite and non-negative")
            }
            Self::InvalidOrigin => formatter.write_str("flex origin must be finite"),
            Self::InvalidGap => formatter.write_str("flex gap must be finite and non-negative"),
            Self::InvalidCrossSize => {
                formatter.write_str("flex cross size constraints must be finite and non-negative")
            }
            Self::InvalidItemAlignmentCount { expected, actual } => write!(
                formatter,
                "flex item alignment count mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidItemSize { node } => {
                write!(formatter, "flex item {node:?} has an invalid base size")
            }
            Self::InvalidMargin { node } => {
                write!(formatter, "flex item {node:?} has a non-finite margin")
            }
            Self::NegativeMarginUnsupported { node } => write!(
                formatter,
                "flex item {node:?} uses a negative margin outside the first row slice"
            ),
            Self::InvalidFlexFactor { node } => {
                write!(
                    formatter,
                    "flex item {node:?} has an invalid grow or shrink factor"
                )
            }
            Self::InvalidFlexMainSizeLimit { node } => {
                write!(
                    formatter,
                    "flex item {node:?} has an invalid main-size limit"
                )
            }
            Self::FlexMainSizeLimitRequiresRedistribution { node } => write!(
                formatter,
                "flex item {node:?} reached a main-size limit before freeze redistribution is available"
            ),
            Self::ShrinkWouldProduceNegativeSize { node } => write!(
                formatter,
                "flex item {node:?} would shrink below zero before a later min-size slice"
            ),
            Self::GeometryOverflow => {
                formatter.write_str("flex layout geometry overflowed the finite coordinate range")
            }
        }
    }
}

impl std::error::Error for FlexLayoutError {}

pub fn layout_flexible_single_line_flex_row(
    origin: Point,
    available_size: Size,
    items: &[FlexibleFlexRowItem],
) -> Result<FlexRowLayout, FlexLayoutError> {
    layout_flexible_single_line_flex_row_with_options(
        origin,
        available_size,
        items,
        FlexRowOptions::default(),
    )
}

pub fn layout_flexible_single_line_flex_row_with_alignment(
    origin: Point,
    available_size: Size,
    items: &[FlexibleFlexRowItem],
    alignment: FlexMainAlignment,
) -> Result<FlexRowLayout, FlexLayoutError> {
    layout_flexible_single_line_flex_row_with_options(
        origin,
        available_size,
        items,
        FlexRowOptions::default().with_main_alignment(alignment),
    )
}

pub fn layout_flexible_single_line_flex_row_with_options(
    origin: Point,
    available_size: Size,
    items: &[FlexibleFlexRowItem],
    options: FlexRowOptions,
) -> Result<FlexRowLayout, FlexLayoutError> {
    layout_flexible_single_line_flex_row_with_item_alignments(
        origin,
        available_size,
        items,
        options,
        &[],
    )
}

pub fn layout_flexible_single_line_flex_row_with_item_alignments(
    origin: Point,
    available_size: Size,
    items: &[FlexibleFlexRowItem],
    options: FlexRowOptions,
    item_cross_alignments: &[Option<FlexCrossAlignment>],
) -> Result<FlexRowLayout, FlexLayoutError> {
    validate_origin(origin)?;
    validate_size(available_size).map_err(|_| FlexLayoutError::InvalidAvailableSize)?;
    validate_options(options)?;
    validate_item_alignment_count(items.len(), item_cross_alignments)?;

    let mut outer_base_width = main_gap_extent(items.len(), options.main_gap())?;
    let mut total_grow = 0.0_f32;
    let mut total_shrink = 0.0_f32;
    for flexible in items {
        validate_item(flexible.item)?;
        validate_flex_factors(*flexible)?;
        outer_base_width = finite_add(outer_base_width, flexible.item.margin.left)?;
        outer_base_width = finite_add(outer_base_width, flexible.item.base_size.width)?;
        outer_base_width = finite_add(outer_base_width, flexible.item.margin.right)?;
        total_grow = finite_add(total_grow, flexible.grow)?;
        total_shrink = finite_add(total_shrink, flexible.shrink)?;
    }

    let free_space = available_size.width - outer_base_width;
    if !free_space.is_finite() {
        return Err(FlexLayoutError::GeometryOverflow);
    }

    let mut resolved = items.iter().map(|item| item.item).collect::<Vec<_>>();
    if free_space > 0.0 && total_grow > 0.0 {
        let distributable = if total_grow < 1.0 {
            finite_mul(free_space, total_grow)?
        } else {
            free_space
        };
        for (target, flexible) in resolved.iter_mut().zip(items) {
            if flexible.grow == 0.0 {
                continue;
            }
            let share = finite_mul(distributable, flexible.grow)? / total_grow;
            let width = finite_add(target.base_size.width, share)?;
            validate_flexible_width(*flexible, width)?;
            target.base_size.width = width;
        }
    } else if free_space < 0.0 && total_shrink > 0.0 {
        let deficit = -free_space;
        let distributable = if total_shrink < 1.0 {
            finite_mul(deficit, total_shrink)?
        } else {
            deficit
        };
        let mut scaled_total = 0.0_f32;
        let mut scaled = Vec::with_capacity(items.len());
        for flexible in items {
            let factor = finite_mul(flexible.shrink, flexible.item.base_size.width)?;
            scaled_total = finite_add(scaled_total, factor)?;
            scaled.push(factor);
        }
        if scaled_total > 0.0 {
            for ((target, flexible), factor) in resolved.iter_mut().zip(items).zip(scaled) {
                if factor == 0.0 {
                    continue;
                }
                let reduction = finite_mul(distributable, factor)? / scaled_total;
                let width = target.base_size.width - reduction;
                let tolerance = f32::EPSILON * target.base_size.width.max(1.0) * 4.0;
                if width < -tolerance {
                    return Err(FlexLayoutError::ShrinkWouldProduceNegativeSize {
                        node: flexible.item.node,
                    });
                }
                let width = width.max(0.0);
                validate_flexible_width(*flexible, width)?;
                target.base_size.width = width;
            }
        }
    }

    layout_single_line_flex_row_with_item_alignments(
        origin,
        available_size,
        &resolved,
        options,
        item_cross_alignments,
    )
}

pub fn layout_wrapped_flexible_rows_with_item_alignments(
    origin: Point,
    available_size: Size,
    items: &[FlexibleFlexRowItem],
    options: FlexRowOptions,
    item_cross_alignments: &[Option<FlexCrossAlignment>],
) -> Result<FlexMultiLineLayout, FlexLayoutError> {
    validate_origin(origin)?;
    validate_size(available_size).map_err(|_| FlexLayoutError::InvalidAvailableSize)?;
    validate_options(options)?;
    validate_item_alignment_count(items.len(), item_cross_alignments)?;

    if items.is_empty() {
        let used_cross_size = resolve_cross_size(0.0, options)?;
        return Ok(FlexMultiLineLayout {
            items: Vec::new(),
            content_size: Size {
                width: 0.0,
                height: used_cross_size,
            },
            line_count: 0,
            overflows_main_axis: false,
            overflows_cross_axis: used_cross_size > available_size.height,
        });
    }

    let mut ranges = Vec::new();
    let mut line_start = 0usize;
    let mut line_main_size = 0.0_f32;
    for (index, flexible) in items.iter().enumerate() {
        validate_item(flexible.item)?;
        validate_flex_factors(*flexible)?;
        let outer_main_size = flex_item_outer_main_size(flexible.item)?;
        let candidate = if index == line_start {
            outer_main_size
        } else {
            finite_add(
                finite_add(line_main_size, options.main_gap())?,
                outer_main_size,
            )?
        };
        if index > line_start && candidate > available_size.width {
            ranges.push((line_start, index));
            line_start = index;
            line_main_size = outer_main_size;
        } else {
            line_main_size = candidate;
        }
    }
    ranges.push((line_start, items.len()));

    let natural_line_options = FlexRowOptions {
        cross_gap: 0.0,
        cross_size: None,
        min_cross_size: None,
        max_cross_size: None,
        ..options
    };
    let mut line_cross_sizes = Vec::with_capacity(ranges.len());
    let mut max_main_size = 0.0_f32;
    let mut overflows_main_axis = false;
    for (start, end) in ranges.iter().copied() {
        let alignments = if item_cross_alignments.is_empty() {
            &[][..]
        } else {
            &item_cross_alignments[start..end]
        };
        let row = layout_flexible_single_line_flex_row_with_item_alignments(
            origin,
            available_size,
            &items[start..end],
            natural_line_options,
            alignments,
        )?;
        line_cross_sizes.push(row.content_size.height);
        max_main_size = max_main_size.max(row.content_size.width);
        overflows_main_axis |= row.overflows_main_axis;
    }

    let mut natural_cross_size = main_gap_extent(ranges.len(), options.cross_gap())?;
    for line_cross_size in &line_cross_sizes {
        natural_cross_size = finite_add(natural_cross_size, *line_cross_size)?;
    }
    let used_cross_size = resolve_cross_size(natural_cross_size, options)?;
    let remaining = used_cross_size - natural_cross_size;
    if !remaining.is_finite() {
        return Err(FlexLayoutError::GeometryOverflow);
    }
    let (leading, distributed_gap, stretch) =
        resolve_content_distribution(remaining, ranges.len(), options.content_alignment());

    let mut placements = Vec::with_capacity(items.len());
    let mut y = finite_add(origin.y, leading)?;
    for (line_index, (start, end)) in ranges.iter().copied().enumerate() {
        let alignments = if item_cross_alignments.is_empty() {
            &[][..]
        } else {
            &item_cross_alignments[start..end]
        };
        let line_cross_size = finite_add(line_cross_sizes[line_index], stretch)?;
        let row = layout_flexible_single_line_flex_row_with_item_alignments(
            Point { x: origin.x, y },
            available_size,
            &items[start..end],
            natural_line_options.with_cross_size(Some(line_cross_size)),
            alignments,
        )?;
        placements.extend(row.items);
        y = finite_add(y, line_cross_size)?;
        if line_index + 1 < ranges.len() {
            y = finite_add(y, options.cross_gap())?;
            y = finite_add(y, distributed_gap)?;
        }
    }

    Ok(FlexMultiLineLayout {
        items: placements,
        content_size: Size {
            width: max_main_size,
            height: used_cross_size,
        },
        line_count: ranges.len(),
        overflows_main_axis,
        overflows_cross_axis: natural_cross_size > used_cross_size
            || used_cross_size > available_size.height,
    })
}

pub fn layout_single_line_flex_row(
    origin: Point,
    available_size: Size,
    items: &[FlexRowItem],
) -> Result<FlexRowLayout, FlexLayoutError> {
    layout_single_line_flex_row_with_options(
        origin,
        available_size,
        items,
        FlexRowOptions::default(),
    )
}

pub fn layout_single_line_flex_row_with_alignment(
    origin: Point,
    available_size: Size,
    items: &[FlexRowItem],
    alignment: FlexMainAlignment,
) -> Result<FlexRowLayout, FlexLayoutError> {
    layout_single_line_flex_row_with_options(
        origin,
        available_size,
        items,
        FlexRowOptions::default().with_main_alignment(alignment),
    )
}

pub fn layout_single_line_flex_row_with_options(
    origin: Point,
    available_size: Size,
    items: &[FlexRowItem],
    options: FlexRowOptions,
) -> Result<FlexRowLayout, FlexLayoutError> {
    layout_single_line_flex_row_with_item_alignments(origin, available_size, items, options, &[])
}

pub fn layout_single_line_flex_row_with_item_alignments(
    origin: Point,
    available_size: Size,
    items: &[FlexRowItem],
    options: FlexRowOptions,
    item_cross_alignments: &[Option<FlexCrossAlignment>],
) -> Result<FlexRowLayout, FlexLayoutError> {
    validate_origin(origin)?;
    validate_size(available_size).map_err(|_| FlexLayoutError::InvalidAvailableSize)?;
    validate_options(options)?;
    validate_item_alignment_count(items.len(), item_cross_alignments)?;

    let mut cursor_x = origin.x;
    let mut max_cross_size = 0.0_f32;
    let mut placements = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        validate_item(*item)?;

        cursor_x = finite_add(cursor_x, item.margin.left)?;
        let border_y = finite_add(origin.y, item.margin.top)?;
        let border_box = Rect {
            origin: Point {
                x: cursor_x,
                y: border_y,
            },
            size: item.base_size,
        };
        placements.push(FlexRowPlacement {
            node: item.node,
            border_box,
        });

        cursor_x = finite_add(cursor_x, item.base_size.width)?;
        cursor_x = finite_add(cursor_x, item.margin.right)?;
        if index + 1 < items.len() {
            cursor_x = finite_add(cursor_x, options.main_gap())?;
        }
        let cross_size = finite_add(item.margin.top, item.base_size.height)?;
        let cross_size = finite_add(cross_size, item.margin.bottom)?;
        max_cross_size = max_cross_size.max(cross_size);
    }

    let used_main_size = cursor_x - origin.x;
    if !used_main_size.is_finite() {
        return Err(FlexLayoutError::GeometryOverflow);
    }
    let used_main_size = used_main_size.max(0.0);
    let mut layout = FlexRowLayout {
        items: placements,
        content_size: Size {
            width: used_main_size,
            height: max_cross_size,
        },
        overflows_main_axis: used_main_size > available_size.width,
        overflows_cross_axis: max_cross_size > available_size.height,
    };
    apply_main_alignment(&mut layout, available_size.width, options.main_alignment())?;
    let used_cross_size = resolve_cross_size(max_cross_size, options)?;
    apply_cross_alignment(
        &mut layout,
        items,
        origin.y,
        used_cross_size,
        options.cross_alignment(),
        item_cross_alignments,
    )?;
    Ok(layout)
}

fn apply_cross_alignment(
    layout: &mut FlexRowLayout,
    items: &[FlexRowItem],
    origin_y: f32,
    cross_size: f32,
    container_alignment: FlexCrossAlignment,
    item_cross_alignments: &[Option<FlexCrossAlignment>],
) -> Result<(), FlexLayoutError> {
    for (index, (placement, item)) in layout.items.iter_mut().zip(items).enumerate() {
        let alignment = item_cross_alignments
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(container_alignment);
        let outer_height = finite_add(item.margin.top, item.base_size.height)?;
        let outer_height = finite_add(outer_height, item.margin.bottom)?;
        let remaining = cross_size - outer_height;
        if !remaining.is_finite() {
            return Err(FlexLayoutError::GeometryOverflow);
        }
        let offset = match alignment {
            FlexCrossAlignment::Start | FlexCrossAlignment::Stretch => 0.0,
            FlexCrossAlignment::End => remaining,
            FlexCrossAlignment::Center => remaining / 2.0,
        };
        let border_y = finite_add(origin_y, offset)?;
        placement.border_box.origin.y = finite_add(border_y, item.margin.top)?;
    }
    Ok(())
}

fn resolve_cross_size(
    natural_cross_size: f32,
    options: FlexRowOptions,
) -> Result<f32, FlexLayoutError> {
    let mut used = options.cross_size().unwrap_or(natural_cross_size).max(0.0);
    if let Some(maximum) = options.max_cross_size() {
        used = used.min(maximum.max(0.0));
    }
    if let Some(minimum) = options.min_cross_size() {
        used = used.max(minimum.max(0.0));
    }
    used.is_finite()
        .then_some(used)
        .ok_or(FlexLayoutError::GeometryOverflow)
}

fn resolve_content_distribution(
    remaining: f32,
    count: usize,
    alignment: FlexContentAlignment,
) -> (f32, f32, f32) {
    if remaining >= 0.0 {
        match alignment {
            FlexContentAlignment::Stretch if count > 0 => (0.0, 0.0, remaining / count as f32),
            FlexContentAlignment::Stretch | FlexContentAlignment::Start => (0.0, 0.0, 0.0),
            FlexContentAlignment::End => (remaining, 0.0, 0.0),
            FlexContentAlignment::Center => (remaining / 2.0, 0.0, 0.0),
            FlexContentAlignment::SpaceBetween if count > 1 => {
                (0.0, remaining / (count - 1) as f32, 0.0)
            }
            FlexContentAlignment::SpaceBetween => (0.0, 0.0, 0.0),
            FlexContentAlignment::SpaceAround if count > 0 => {
                let gap = remaining / count as f32;
                (gap / 2.0, gap, 0.0)
            }
            FlexContentAlignment::SpaceAround => (0.0, 0.0, 0.0),
            FlexContentAlignment::SpaceEvenly if count > 0 => {
                let gap = remaining / (count + 1) as f32;
                (gap, gap, 0.0)
            }
            FlexContentAlignment::SpaceEvenly => (0.0, 0.0, 0.0),
        }
    } else {
        match alignment {
            FlexContentAlignment::End => (remaining, 0.0, 0.0),
            FlexContentAlignment::Center => (remaining / 2.0, 0.0, 0.0),
            FlexContentAlignment::Stretch
            | FlexContentAlignment::Start
            | FlexContentAlignment::SpaceBetween
            | FlexContentAlignment::SpaceAround
            | FlexContentAlignment::SpaceEvenly => (0.0, 0.0, 0.0),
        }
    }
}

fn apply_main_alignment(
    layout: &mut FlexRowLayout,
    available_width: f32,
    alignment: FlexMainAlignment,
) -> Result<(), FlexLayoutError> {
    let remaining = available_width - layout.content_size.width;
    if !remaining.is_finite() {
        return Err(FlexLayoutError::GeometryOverflow);
    }

    let count = layout.items.len();
    let (leading, gap) = if remaining >= 0.0 {
        match alignment {
            FlexMainAlignment::Start => (0.0, 0.0),
            FlexMainAlignment::End => (remaining, 0.0),
            FlexMainAlignment::Center => (remaining / 2.0, 0.0),
            FlexMainAlignment::SpaceBetween if count > 1 => (0.0, remaining / (count - 1) as f32),
            FlexMainAlignment::SpaceBetween => (0.0, 0.0),
            FlexMainAlignment::SpaceAround if count > 0 => {
                let gap = remaining / count as f32;
                (gap / 2.0, gap)
            }
            FlexMainAlignment::SpaceAround => (0.0, 0.0),
            FlexMainAlignment::SpaceEvenly if count > 0 => {
                let gap = remaining / (count + 1) as f32;
                (gap, gap)
            }
            FlexMainAlignment::SpaceEvenly => (0.0, 0.0),
        }
    } else {
        match alignment {
            FlexMainAlignment::End => (remaining, 0.0),
            FlexMainAlignment::Center => (remaining / 2.0, 0.0),
            FlexMainAlignment::Start
            | FlexMainAlignment::SpaceBetween
            | FlexMainAlignment::SpaceAround
            | FlexMainAlignment::SpaceEvenly => (0.0, 0.0),
        }
    };

    let mut accumulated_gap = leading;
    for (index, placement) in layout.items.iter_mut().enumerate() {
        placement.border_box.origin.x = finite_add(placement.border_box.origin.x, accumulated_gap)?;
        if index + 1 < count {
            accumulated_gap = finite_add(accumulated_gap, gap)?;
        }
    }
    Ok(())
}

fn validate_item_alignment_count(
    item_count: usize,
    item_cross_alignments: &[Option<FlexCrossAlignment>],
) -> Result<(), FlexLayoutError> {
    if item_cross_alignments.is_empty() || item_cross_alignments.len() == item_count {
        Ok(())
    } else {
        Err(FlexLayoutError::InvalidItemAlignmentCount {
            expected: item_count,
            actual: item_cross_alignments.len(),
        })
    }
}

fn validate_options(options: FlexRowOptions) -> Result<(), FlexLayoutError> {
    if !options.main_gap().is_finite()
        || options.main_gap() < 0.0
        || !options.cross_gap().is_finite()
        || options.cross_gap() < 0.0
    {
        return Err(FlexLayoutError::InvalidGap);
    }

    for size in [
        options.cross_size(),
        options.min_cross_size(),
        options.max_cross_size(),
    ]
    .into_iter()
    .flatten()
    {
        if !size.is_finite() || size < 0.0 {
            return Err(FlexLayoutError::InvalidCrossSize);
        }
    }
    Ok(())
}

fn flex_item_outer_main_size(item: FlexRowItem) -> Result<f32, FlexLayoutError> {
    let size = finite_add(item.margin.left, item.base_size.width)?;
    finite_add(size, item.margin.right)
}

fn main_gap_extent(count: usize, gap: f32) -> Result<f32, FlexLayoutError> {
    if count <= 1 {
        return Ok(0.0);
    }
    finite_mul(gap, (count - 1) as f32)
}

fn finite_mul(left: f32, right: f32) -> Result<f32, FlexLayoutError> {
    let value = left * right;
    value
        .is_finite()
        .then_some(value)
        .ok_or(FlexLayoutError::GeometryOverflow)
}

fn finite_add(left: f32, right: f32) -> Result<f32, FlexLayoutError> {
    let value = left + right;
    value
        .is_finite()
        .then_some(value)
        .ok_or(FlexLayoutError::GeometryOverflow)
}

fn validate_flex_factors(item: FlexibleFlexRowItem) -> Result<(), FlexLayoutError> {
    if !item.grow.is_finite() || !item.shrink.is_finite() || item.grow < 0.0 || item.shrink < 0.0 {
        return Err(FlexLayoutError::InvalidFlexFactor {
            node: item.item.node,
        });
    }
    if !item.min_main_size.is_finite()
        || item.min_main_size < 0.0
        || item
            .max_main_size
            .is_some_and(|max| !max.is_finite() || max < item.min_main_size)
    {
        return Err(FlexLayoutError::InvalidFlexMainSizeLimit {
            node: item.item.node,
        });
    }
    Ok(())
}

fn validate_flexible_width(item: FlexibleFlexRowItem, width: f32) -> Result<(), FlexLayoutError> {
    let tolerance = f32::EPSILON * item.item.base_size.width.max(1.0) * 4.0;
    let below_min = width + tolerance < item.min_main_size;
    let above_max = item
        .max_main_size
        .is_some_and(|max| width - tolerance > max);
    if below_min || above_max {
        Err(FlexLayoutError::FlexMainSizeLimitRequiresRedistribution {
            node: item.item.node,
        })
    } else {
        Ok(())
    }
}

fn validate_origin(origin: Point) -> Result<(), FlexLayoutError> {
    if origin.x.is_finite() && origin.y.is_finite() {
        Ok(())
    } else {
        Err(FlexLayoutError::InvalidOrigin)
    }
}

fn validate_size(size: Size) -> Result<(), ()> {
    if size.width.is_finite() && size.height.is_finite() && size.width >= 0.0 && size.height >= 0.0
    {
        Ok(())
    } else {
        Err(())
    }
}

fn validate_item(item: FlexRowItem) -> Result<(), FlexLayoutError> {
    validate_size(item.base_size)
        .map_err(|_| FlexLayoutError::InvalidItemSize { node: item.node })?;
    let margins = [
        item.margin.top,
        item.margin.right,
        item.margin.bottom,
        item.margin.left,
    ];
    if margins.iter().any(|value| !value.is_finite()) {
        return Err(FlexLayoutError::InvalidMargin { node: item.node });
    }
    if margins.iter().any(|value| *value < 0.0) {
        return Err(FlexLayoutError::NegativeMarginUnsupported { node: item.node });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(index: usize, width: f32, height: f32) -> FlexRowItem {
        FlexRowItem::new(LayoutNodeId(index), Size { width, height }, EdgeSizes::ZERO)
    }

    #[test]
    fn row_places_fixed_items_in_source_order() {
        let layout = layout_single_line_flex_row(
            Point { x: 10.0, y: 20.0 },
            Size {
                width: 100.0,
                height: 50.0,
            },
            &[item(1, 20.0, 10.0), item(2, 30.0, 15.0)],
        )
        .unwrap();

        assert_eq!(layout.items.len(), 2);
        assert_eq!(
            layout.items[0].border_box,
            Rect {
                origin: Point { x: 10.0, y: 20.0 },
                size: Size {
                    width: 20.0,
                    height: 10.0,
                },
            }
        );
        assert_eq!(
            layout.items[1].border_box.origin,
            Point { x: 30.0, y: 20.0 }
        );
        assert_eq!(
            layout.content_size,
            Size {
                width: 50.0,
                height: 15.0,
            }
        );
        assert!(!layout.overflows_main_axis);
        assert!(!layout.overflows_cross_axis);
    }

    #[test]
    fn margins_participate_in_main_and_cross_extent() {
        let layout = layout_single_line_flex_row(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[FlexRowItem::new(
                LayoutNodeId(1),
                Size {
                    width: 20.0,
                    height: 10.0,
                },
                EdgeSizes::new(2.0, 4.0, 6.0, 8.0),
            )],
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.origin, Point { x: 8.0, y: 2.0 });
        assert_eq!(layout.content_size.width, 32.0);
        assert_eq!(layout.content_size.height, 18.0);
    }

    #[test]
    fn overflow_is_reported_without_silently_resizing_items() {
        let layout = layout_single_line_flex_row(
            Point::default(),
            Size {
                width: 40.0,
                height: 12.0,
            },
            &[item(1, 25.0, 10.0), item(2, 25.0, 20.0)],
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 25.0);
        assert_eq!(layout.items[1].border_box.size.width, 25.0);
        assert!(layout.overflows_main_axis);
        assert!(layout.overflows_cross_axis);
    }

    #[test]
    fn wrapped_rows_break_at_available_width_and_use_cross_gap() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 40.0, 10.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(2, 40.0, 20.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(3, 40.0, 15.0), 0.0, 0.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default()
                .with_main_gap(10.0)
                .with_cross_gap(5.0),
            &[],
        )
        .unwrap();

        assert_eq!(layout.line_count, 2);
        assert_eq!(layout.items[0].border_box.origin, Point { x: 0.0, y: 0.0 });
        assert_eq!(layout.items[1].border_box.origin, Point { x: 50.0, y: 0.0 });
        assert_eq!(layout.items[2].border_box.origin, Point { x: 0.0, y: 25.0 });
        assert_eq!(layout.content_size.height, 40.0);
    }

    #[test]
    fn wrapped_rows_distribute_grow_independently_per_line() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 30.0, 10.0), 1.0, 1.0),
            FlexibleFlexRowItem::new(item(2, 30.0, 10.0), 1.0, 1.0),
            FlexibleFlexRowItem::new(item(3, 30.0, 10.0), 1.0, 1.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 80.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default().with_main_gap(10.0),
            &[],
        )
        .unwrap();

        assert_eq!(layout.line_count, 2);
        assert_eq!(layout.items[0].border_box.size.width, 35.0);
        assert_eq!(layout.items[1].border_box.size.width, 35.0);
        assert_eq!(layout.items[1].border_box.origin.x, 45.0);
        assert_eq!(layout.items[2].border_box.size.width, 80.0);
        assert_eq!(layout.items[2].border_box.origin.x, 0.0);
    }

    #[test]
    fn wrapped_rows_apply_per_line_alignment_and_item_cross_overrides() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 45.0, 10.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(2, 45.0, 20.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(3, 45.0, 10.0), 0.0, 0.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default()
                .with_main_alignment(FlexMainAlignment::Center)
                .with_cross_alignment(FlexCrossAlignment::Center),
            &[Some(FlexCrossAlignment::End), None, None],
        )
        .unwrap();

        assert_eq!(layout.line_count, 2);
        assert_eq!(layout.items[0].border_box.origin, Point { x: 5.0, y: 10.0 });
        assert_eq!(layout.items[1].border_box.origin, Point { x: 50.0, y: 0.0 });
        assert_eq!(
            layout.items[2].border_box.origin,
            Point { x: 27.5, y: 20.0 }
        );
    }

    #[test]
    fn wrapped_rows_stretch_lines_into_definite_cross_size() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 60.0, 10.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(2, 60.0, 20.0), 0.0, 0.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default().with_cross_size(Some(60.0)),
            &[],
        )
        .unwrap();

        assert_eq!(layout.line_count, 2);
        assert_eq!(layout.content_size.height, 60.0);
        assert_eq!(layout.items[0].border_box.origin.y, 0.0);
        assert_eq!(layout.items[1].border_box.origin.y, 25.0);
    }

    #[test]
    fn wrapped_rows_center_lines_in_definite_cross_size() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 60.0, 10.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(2, 60.0, 20.0), 0.0, 0.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default()
                .with_content_alignment(FlexContentAlignment::Center)
                .with_cross_size(Some(60.0)),
            &[],
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.origin.y, 15.0);
        assert_eq!(layout.items[1].border_box.origin.y, 25.0);
    }

    #[test]
    fn wrapped_rows_space_between_adds_to_fixed_cross_gap() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 60.0, 10.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(2, 60.0, 20.0), 0.0, 0.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default()
                .with_content_alignment(FlexContentAlignment::SpaceBetween)
                .with_cross_gap(5.0)
                .with_cross_size(Some(60.0)),
            &[],
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.origin.y, 0.0);
        assert_eq!(layout.items[1].border_box.origin.y, 40.0);
        assert_eq!(layout.content_size.height, 60.0);
    }

    #[test]
    fn wrapped_rows_use_cross_size_limits_for_content_distribution() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 60.0, 10.0), 0.0, 0.0),
            FlexibleFlexRowItem::new(item(2, 60.0, 20.0), 0.0, 0.0),
        ];
        let layout = layout_wrapped_flexible_rows_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default()
                .with_content_alignment(FlexContentAlignment::End)
                .with_cross_size_limits(Some(50.0), None),
            &[],
        )
        .unwrap();

        assert_eq!(layout.content_size.height, 50.0);
        assert_eq!(layout.items[0].border_box.origin.y, 20.0);
        assert_eq!(layout.items[1].border_box.origin.y, 30.0);
    }

    #[test]
    fn per_item_cross_alignment_overrides_container_alignment() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 20.0)];
        let layout = layout_single_line_flex_row_with_item_alignments(
            Point::default(),
            Size {
                width: 100.0,
                height: 60.0,
            },
            &items,
            FlexRowOptions::default()
                .with_cross_alignment(FlexCrossAlignment::Center)
                .with_cross_size(Some(60.0)),
            &[Some(FlexCrossAlignment::End), None],
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.origin.y, 50.0);
        assert_eq!(layout.items[1].border_box.origin.y, 20.0);
    }

    #[test]
    fn item_cross_alignment_count_must_match_when_overrides_are_present() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 20.0)];
        assert_eq!(
            layout_single_line_flex_row_with_item_alignments(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 60.0,
                },
                &items,
                FlexRowOptions::default(),
                &[Some(FlexCrossAlignment::End)],
            ),
            Err(FlexLayoutError::InvalidItemAlignmentCount {
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn cross_alignment_uses_natural_line_size_for_auto_cross_size() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 30.0)];
        let end = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default().with_cross_alignment(FlexCrossAlignment::End),
        )
        .unwrap();
        assert_eq!(end.items[0].border_box.origin.y, 20.0);
        assert_eq!(end.items[1].border_box.origin.y, 0.0);
        assert_eq!(end.content_size.height, 30.0);

        let center = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &items,
            FlexRowOptions::default().with_cross_alignment(FlexCrossAlignment::Center),
        )
        .unwrap();
        assert_eq!(center.items[0].border_box.origin.y, 10.0);
        assert_eq!(center.items[1].border_box.origin.y, 0.0);
    }

    #[test]
    fn cross_alignment_respects_definite_and_minimum_cross_sizes() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 20.0)];
        let definite = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 60.0,
            },
            &items,
            FlexRowOptions::default()
                .with_cross_alignment(FlexCrossAlignment::Center)
                .with_cross_size(Some(60.0)),
        )
        .unwrap();
        assert_eq!(definite.items[0].border_box.origin.y, 25.0);
        assert_eq!(definite.items[1].border_box.origin.y, 20.0);

        let minimum = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 60.0,
            },
            &items,
            FlexRowOptions::default()
                .with_cross_alignment(FlexCrossAlignment::End)
                .with_cross_size_limits(Some(50.0), None),
        )
        .unwrap();
        assert_eq!(minimum.items[0].border_box.origin.y, 40.0);
        assert_eq!(minimum.items[1].border_box.origin.y, 30.0);
    }

    #[test]
    fn stretch_preserves_explicit_cross_sizes_in_the_current_slice() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 20.0)];
        let layout = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 60.0,
            },
            &items,
            FlexRowOptions::default()
                .with_cross_alignment(FlexCrossAlignment::Stretch)
                .with_cross_size(Some(60.0)),
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.origin.y, 0.0);
        assert_eq!(layout.items[0].border_box.size.height, 10.0);
        assert_eq!(layout.items[1].border_box.size.height, 20.0);
    }

    #[test]
    fn invalid_cross_size_options_are_rejected_explicitly() {
        let items = [item(1, 20.0, 10.0)];
        assert_eq!(
            layout_single_line_flex_row_with_options(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &items,
                FlexRowOptions::default().with_cross_size(Some(f32::NAN)),
            ),
            Err(FlexLayoutError::InvalidCrossSize)
        );
        assert_eq!(
            layout_single_line_flex_row_with_options(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &items,
                FlexRowOptions::default().with_cross_size_limits(Some(-1.0), None),
            ),
            Err(FlexLayoutError::InvalidCrossSize)
        );
    }

    #[test]
    fn main_gap_participates_in_placement_and_content_extent() {
        let items = [item(1, 20.0, 10.0), item(2, 30.0, 10.0)];
        let layout = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 20.0,
            },
            &items,
            FlexRowOptions::default().with_main_gap(10.0),
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.origin.x, 0.0);
        assert_eq!(layout.items[1].border_box.origin.x, 30.0);
        assert_eq!(layout.content_size.width, 60.0);
    }

    #[test]
    fn flexible_sizing_reserves_fixed_gap_before_grow_distribution() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 20.0, 10.0), 1.0, 1.0),
            FlexibleFlexRowItem::new(item(2, 20.0, 10.0), 1.0, 1.0),
        ];
        let layout = layout_flexible_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 20.0,
            },
            &items,
            FlexRowOptions::default().with_main_gap(10.0),
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 45.0);
        assert_eq!(layout.items[1].border_box.size.width, 45.0);
        assert_eq!(layout.items[1].border_box.origin.x, 55.0);
        assert_eq!(layout.content_size.width, 100.0);
    }

    #[test]
    fn fixed_gap_and_distributed_alignment_compose_after_sizing() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 10.0)];
        let layout = layout_single_line_flex_row_with_options(
            Point::default(),
            Size {
                width: 100.0,
                height: 20.0,
            },
            &items,
            FlexRowOptions::default()
                .with_main_gap(10.0)
                .with_main_alignment(FlexMainAlignment::Center),
        )
        .unwrap();

        assert_eq!(layout.content_size.width, 50.0);
        assert_eq!(layout.items[0].border_box.origin.x, 25.0);
        assert_eq!(layout.items[1].border_box.origin.x, 55.0);
    }

    #[test]
    fn invalid_main_gap_is_rejected_explicitly() {
        let items = [item(1, 20.0, 10.0)];
        assert_eq!(
            layout_single_line_flex_row_with_options(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &items,
                FlexRowOptions::default().with_main_gap(f32::NAN),
            ),
            Err(FlexLayoutError::InvalidGap)
        );
        assert_eq!(
            layout_single_line_flex_row_with_options(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &items,
                FlexRowOptions::default().with_main_gap(-1.0),
            ),
            Err(FlexLayoutError::InvalidGap)
        );
    }

    #[test]
    fn main_axis_alignment_positions_remaining_free_space() {
        let items = [item(1, 20.0, 10.0), item(2, 20.0, 10.0)];
        let available = Size {
            width: 100.0,
            height: 20.0,
        };

        let end = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::End,
        )
        .unwrap();
        assert_eq!(end.items[0].border_box.origin.x, 60.0);
        assert_eq!(end.items[1].border_box.origin.x, 80.0);

        let center = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::Center,
        )
        .unwrap();
        assert_eq!(center.items[0].border_box.origin.x, 30.0);
        assert_eq!(center.items[1].border_box.origin.x, 50.0);

        let between = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::SpaceBetween,
        )
        .unwrap();
        assert_eq!(between.items[0].border_box.origin.x, 0.0);
        assert_eq!(between.items[1].border_box.origin.x, 80.0);

        let around = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::SpaceAround,
        )
        .unwrap();
        assert_eq!(around.items[0].border_box.origin.x, 15.0);
        assert_eq!(around.items[1].border_box.origin.x, 65.0);

        let evenly = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::SpaceEvenly,
        )
        .unwrap();
        assert_eq!(evenly.items[0].border_box.origin.x, 20.0);
        assert_eq!(evenly.items[1].border_box.origin.x, 60.0);
    }

    #[test]
    fn distributed_alignment_uses_residual_space_after_partial_grow() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 20.0, 10.0), 0.25, 1.0),
            FlexibleFlexRowItem::new(item(2, 20.0, 10.0), 0.25, 1.0),
        ];
        let layout = layout_flexible_single_line_flex_row_with_alignment(
            Point::default(),
            Size {
                width: 100.0,
                height: 20.0,
            },
            &items,
            FlexMainAlignment::SpaceBetween,
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 35.0);
        assert_eq!(layout.items[1].border_box.size.width, 35.0);
        assert_eq!(layout.items[0].border_box.origin.x, 0.0);
        assert_eq!(layout.items[1].border_box.origin.x, 65.0);
        assert_eq!(layout.content_size.width, 70.0);
    }

    #[test]
    fn overflow_alignment_keeps_distributed_values_safe_at_start() {
        let items = [item(1, 40.0, 10.0), item(2, 40.0, 10.0)];
        let available = Size {
            width: 60.0,
            height: 20.0,
        };

        let between = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::SpaceBetween,
        )
        .unwrap();
        assert_eq!(between.items[0].border_box.origin.x, 0.0);

        let end = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::End,
        )
        .unwrap();
        assert_eq!(end.items[0].border_box.origin.x, -20.0);

        let center = layout_single_line_flex_row_with_alignment(
            Point::default(),
            available,
            &items,
            FlexMainAlignment::Center,
        )
        .unwrap();
        assert_eq!(center.items[0].border_box.origin.x, -10.0);
    }

    #[test]
    fn grow_distributes_positive_free_space_by_factor() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 20.0, 10.0), 1.0, 1.0),
            FlexibleFlexRowItem::new(item(2, 20.0, 10.0), 3.0, 1.0),
        ];
        let layout = layout_flexible_single_line_flex_row(
            Point::default(),
            Size {
                width: 100.0,
                height: 20.0,
            },
            &items,
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 35.0);
        assert_eq!(layout.items[1].border_box.size.width, 65.0);
        assert_eq!(layout.content_size.width, 100.0);
        assert!(!layout.overflows_main_axis);
    }

    #[test]
    fn grow_sum_below_one_leaves_part_of_free_space_unclaimed() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 20.0, 10.0), 0.25, 1.0),
            FlexibleFlexRowItem::new(item(2, 20.0, 10.0), 0.25, 1.0),
        ];
        let layout = layout_flexible_single_line_flex_row(
            Point::default(),
            Size {
                width: 100.0,
                height: 20.0,
            },
            &items,
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 35.0);
        assert_eq!(layout.items[1].border_box.size.width, 35.0);
        assert_eq!(layout.content_size.width, 70.0);
    }

    #[test]
    fn shrink_uses_scaled_base_size_factors() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 40.0, 10.0), 0.0, 1.0),
            FlexibleFlexRowItem::new(item(2, 20.0, 10.0), 0.0, 1.0),
        ];
        let layout = layout_flexible_single_line_flex_row(
            Point::default(),
            Size {
                width: 45.0,
                height: 20.0,
            },
            &items,
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 30.0);
        assert_eq!(layout.items[1].border_box.size.width, 15.0);
        assert_eq!(layout.content_size.width, 45.0);
        assert!(!layout.overflows_main_axis);
    }

    #[test]
    fn shrink_sum_below_one_preserves_residual_overflow() {
        let items = [
            FlexibleFlexRowItem::new(item(1, 40.0, 10.0), 0.0, 0.25),
            FlexibleFlexRowItem::new(item(2, 40.0, 10.0), 0.0, 0.25),
        ];
        let layout = layout_flexible_single_line_flex_row(
            Point::default(),
            Size {
                width: 40.0,
                height: 20.0,
            },
            &items,
        )
        .unwrap();

        assert_eq!(layout.items[0].border_box.size.width, 30.0);
        assert_eq!(layout.items[1].border_box.size.width, 30.0);
        assert_eq!(layout.content_size.width, 60.0);
        assert!(layout.overflows_main_axis);
    }

    #[test]
    fn flexible_main_size_limits_fail_closed_until_freeze_redistribution_exists() {
        let grow_limited = [FlexibleFlexRowItem::new(item(1, 20.0, 10.0), 1.0, 1.0)
            .with_main_size_limits(0.0, Some(30.0))];
        assert_eq!(
            layout_flexible_single_line_flex_row(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &grow_limited,
            ),
            Err(FlexLayoutError::FlexMainSizeLimitRequiresRedistribution {
                node: LayoutNodeId(1)
            })
        );

        let shrink_limited = [FlexibleFlexRowItem::new(item(2, 40.0, 10.0), 0.0, 1.0)
            .with_main_size_limits(30.0, None)];
        assert_eq!(
            layout_flexible_single_line_flex_row(
                Point::default(),
                Size {
                    width: 20.0,
                    height: 20.0,
                },
                &shrink_limited,
            ),
            Err(FlexLayoutError::FlexMainSizeLimitRequiresRedistribution {
                node: LayoutNodeId(2)
            })
        );
    }

    #[test]
    fn invalid_factors_and_unresolved_negative_shrink_are_rejected() {
        let invalid = [FlexibleFlexRowItem::new(item(1, 20.0, 10.0), f32::NAN, 1.0)];
        assert_eq!(
            layout_flexible_single_line_flex_row(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &invalid,
            ),
            Err(FlexLayoutError::InvalidFlexFactor {
                node: LayoutNodeId(1)
            })
        );

        let item = FlexRowItem::new(
            LayoutNodeId(2),
            Size {
                width: 10.0,
                height: 10.0,
            },
            EdgeSizes::new(0.0, 40.0, 0.0, 40.0),
        );
        let excessive = [FlexibleFlexRowItem::new(item, 0.0, 1.0)];
        assert_eq!(
            layout_flexible_single_line_flex_row(
                Point::default(),
                Size {
                    width: 0.0,
                    height: 20.0,
                },
                &excessive,
            ),
            Err(FlexLayoutError::ShrinkWouldProduceNegativeSize {
                node: LayoutNodeId(2)
            })
        );
    }

    #[test]
    fn empty_row_has_zero_content_extent() {
        let layout = layout_single_line_flex_row(
            Point { x: 4.0, y: 5.0 },
            Size {
                width: 80.0,
                height: 30.0,
            },
            &[],
        )
        .unwrap();

        assert!(layout.items.is_empty());
        assert_eq!(layout.content_size, Size::default());
    }

    #[test]
    fn invalid_geometry_is_rejected_explicitly() {
        assert_eq!(
            layout_single_line_flex_row(
                Point::default(),
                Size {
                    width: f32::NAN,
                    height: 20.0,
                },
                &[]
            ),
            Err(FlexLayoutError::InvalidAvailableSize)
        );

        let item = FlexRowItem::new(
            LayoutNodeId(9),
            Size {
                width: 10.0,
                height: 10.0,
            },
            EdgeSizes::new(0.0, 0.0, 0.0, -1.0),
        );
        assert_eq!(
            layout_single_line_flex_row(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 20.0,
                },
                &[item]
            ),
            Err(FlexLayoutError::NegativeMarginUnsupported {
                node: LayoutNodeId(9)
            })
        );
    }

    #[test]
    fn finite_inputs_cannot_produce_non_finite_geometry() {
        assert_eq!(
            layout_single_line_flex_row(
                Point::default(),
                Size {
                    width: f32::MAX,
                    height: f32::MAX,
                },
                &[item(1, f32::MAX, 1.0), item(2, f32::MAX, 1.0)]
            ),
            Err(FlexLayoutError::GeometryOverflow)
        );

        assert_eq!(
            layout_single_line_flex_row(
                Point::default(),
                Size {
                    width: f32::MAX,
                    height: f32::MAX,
                },
                &[FlexRowItem::new(
                    LayoutNodeId(3),
                    Size {
                        width: 1.0,
                        height: f32::MAX,
                    },
                    EdgeSizes::new(f32::MAX, 0.0, 0.0, 0.0),
                )]
            ),
            Err(FlexLayoutError::GeometryOverflow)
        );
    }
}
