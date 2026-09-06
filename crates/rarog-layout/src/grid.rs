use crate::LayoutNodeId;
use rarog_types::{Point, Rect, Size};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridAxis {
    Column,
    Row,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridTrack {
    pub base_size: f32,
}

impl GridTrack {
    pub const fn new(base_size: f32) -> Self {
        Self { base_size }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridTrackSizing {
    Fixed(f32),
    Auto,
}

impl GridTrackSizing {
    pub const fn fixed(size: f32) -> Self {
        Self::Fixed(size)
    }

    pub const fn auto() -> Self {
        Self::Auto
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GridTrackGrowthLimit {
    Finite(f32),
    Infinite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridTrackSizingState {
    pub base_size: f32,
    pub growth_limit: GridTrackGrowthLimit,
}

impl GridTrackSizingState {
    const fn fixed(size: f32) -> Self {
        Self {
            base_size: size,
            growth_limit: GridTrackGrowthLimit::Finite(size),
        }
    }

    const fn intrinsic() -> Self {
        Self {
            base_size: 0.0,
            growth_limit: GridTrackGrowthLimit::Infinite,
        }
    }

    fn grow_base_to(&mut self, requested_size: f32) {
        let bounded_size = match self.growth_limit {
            GridTrackGrowthLimit::Finite(limit) => requested_size.min(limit),
            GridTrackGrowthLimit::Infinite => requested_size,
        };
        self.base_size = self.base_size.max(bounded_size);
    }

    const fn resolved_track(self) -> GridTrack {
        GridTrack::new(self.base_size)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridSpanningSizeContribution {
    pub node: LayoutNodeId,
    pub start: usize,
    pub span: usize,
    pub size: f32,
}

impl GridSpanningSizeContribution {
    pub const fn new(node: LayoutNodeId, start: usize, span: usize, size: f32) -> Self {
        Self {
            node,
            start,
            span,
            size,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GridIntrinsicContributionKind {
    Minimum,
    MinContent,
    MaxContent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridAxisIntrinsicContributions {
    pub minimum: f32,
    pub min_content: f32,
    pub max_content: f32,
}

impl GridAxisIntrinsicContributions {
    pub const fn new(minimum: f32, min_content: f32, max_content: f32) -> Self {
        Self {
            minimum,
            min_content,
            max_content,
        }
    }

    const fn size_for(self, kind: GridIntrinsicContributionKind) -> f32 {
        match kind {
            GridIntrinsicContributionKind::Minimum => self.minimum,
            GridIntrinsicContributionKind::MinContent => self.min_content,
            GridIntrinsicContributionKind::MaxContent => self.max_content,
        }
    }

    fn validate(self, node: LayoutNodeId, axis: GridAxis) -> Result<Self, GridLayoutError> {
        if self.minimum.is_finite()
            && self.min_content.is_finite()
            && self.max_content.is_finite()
            && self.minimum >= 0.0
            && self.minimum <= self.min_content
            && self.min_content <= self.max_content
        {
            Ok(self)
        } else {
            Err(GridLayoutError::InvalidContribution { node, axis })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridIntrinsicContributions {
    pub node: LayoutNodeId,
    pub inline: GridAxisIntrinsicContributions,
    pub block: GridAxisIntrinsicContributions,
}

impl GridIntrinsicContributions {
    pub const fn new(
        node: LayoutNodeId,
        inline: GridAxisIntrinsicContributions,
        block: GridAxisIntrinsicContributions,
    ) -> Self {
        Self {
            node,
            inline,
            block,
        }
    }

    const fn from_legacy(contribution: GridItemContribution) -> Self {
        Self {
            node: contribution.node,
            inline: GridAxisIntrinsicContributions::new(
                contribution.inline_size,
                contribution.inline_size,
                contribution.inline_size,
            ),
            block: GridAxisIntrinsicContributions::new(
                contribution.block_size,
                contribution.block_size,
                contribution.block_size,
            ),
        }
    }

    pub(crate) fn size_for(
        self,
        axis: GridAxis,
        kind: GridIntrinsicContributionKind,
    ) -> Result<f32, GridLayoutError> {
        match axis {
            GridAxis::Column => self.inline.validate(self.node, axis),
            GridAxis::Row => self.block.validate(self.node, axis),
        }
        .map(|contributions| contributions.size_for(kind))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridItemContribution {
    pub node: LayoutNodeId,
    pub inline_size: f32,
    pub block_size: f32,
}

impl GridItemContribution {
    pub const fn new(node: LayoutNodeId, inline_size: f32, block_size: f32) -> Self {
        Self {
            node,
            inline_size,
            block_size,
        }
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridItem {
    pub node: LayoutNodeId,
    pub row_start: usize,
    pub column_start: usize,
    pub row_span: usize,
    pub column_span: usize,
}

impl GridItem {
    pub const fn new(node: LayoutNodeId, row_start: usize, column_start: usize) -> Self {
        Self {
            node,
            row_start,
            column_start,
            row_span: 1,
            column_span: 1,
        }
    }

    pub const fn with_span(mut self, row_span: usize, column_span: usize) -> Self {
        self.row_span = row_span;
        self.column_span = column_span;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPlacementRequest {
    pub node: LayoutNodeId,
    pub row_start: Option<usize>,
    pub column_start: Option<usize>,
    pub row_span: usize,
    pub column_span: usize,
}

impl GridPlacementRequest {
    pub const fn auto(node: LayoutNodeId) -> Self {
        Self {
            node,
            row_start: None,
            column_start: None,
            row_span: 1,
            column_span: 1,
        }
    }

    pub const fn explicit(node: LayoutNodeId, row_start: usize, column_start: usize) -> Self {
        Self {
            node,
            row_start: Some(row_start),
            column_start: Some(column_start),
            row_span: 1,
            column_span: 1,
        }
    }

    pub const fn with_row_start(mut self, row_start: Option<usize>) -> Self {
        self.row_start = row_start;
        self
    }

    pub const fn with_column_start(mut self, column_start: Option<usize>) -> Self {
        self.column_start = column_start;
        self
    }

    pub const fn with_span(mut self, row_span: usize, column_span: usize) -> Self {
        self.row_span = row_span;
        self.column_span = column_span;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridPlacement {
    pub node: LayoutNodeId,
    pub area: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridLayout {
    pub items: Vec<GridPlacement>,
    pub content_size: Size,
    pub overflows_columns_axis: bool,
    pub overflows_rows_axis: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridLayoutError {
    InvalidOrigin,
    InvalidAvailableSize,
    InvalidTrackSize { axis: GridAxis, index: usize },
    InvalidContribution { node: LayoutNodeId, axis: GridAxis },
    MissingContribution { node: LayoutNodeId, axis: GridAxis },
    UnsupportedIntrinsicSpan { node: LayoutNodeId, axis: GridAxis },
    InvalidGap { axis: GridAxis },
    InvalidSpan { node: LayoutNodeId },
    PlacementOutsideGrid { node: LayoutNodeId },
    AutoPlacementUnavailable { node: LayoutNodeId },
    GeometryOverflow,
}

impl fmt::Display for GridLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrigin => formatter.write_str("grid origin must be finite"),
            Self::InvalidAvailableSize => {
                formatter.write_str("grid available size must be finite and non-negative")
            }
            Self::InvalidTrackSize { axis, index } => {
                write!(
                    formatter,
                    "grid {axis:?} track {index} must be finite and non-negative"
                )
            }
            Self::InvalidContribution { node, axis } => {
                write!(
                    formatter,
                    "grid item {node:?} has an invalid intrinsic {axis:?} contribution"
                )
            }
            Self::MissingContribution { node, axis } => {
                write!(
                    formatter,
                    "grid item {node:?} is missing an intrinsic {axis:?} contribution"
                )
            }
            Self::UnsupportedIntrinsicSpan { node, axis } => {
                write!(
                    formatter,
                    "grid item {node:?} spans multiple tracks including an intrinsic {axis:?} track"
                )
            }
            Self::InvalidGap { axis } => {
                write!(
                    formatter,
                    "grid {axis:?} gap must be finite and non-negative"
                )
            }
            Self::InvalidSpan { node } => {
                write!(
                    formatter,
                    "grid item {node:?} must span at least one row and column"
                )
            }
            Self::PlacementOutsideGrid { node } => {
                write!(
                    formatter,
                    "grid item {node:?} placement is outside the explicit grid"
                )
            }
            Self::AutoPlacementUnavailable { node } => {
                write!(
                    formatter,
                    "grid item {node:?} cannot be auto-placed in the explicit grid"
                )
            }
            Self::GeometryOverflow => formatter.write_str("grid geometry overflowed finite bounds"),
        }
    }
}

impl std::error::Error for GridLayoutError {}

pub fn layout_fixed_grid(
    origin: Point,
    available_size: Size,
    columns: &[GridTrack],
    rows: &[GridTrack],
    column_gap: f32,
    row_gap: f32,
    items: &[GridItem],
) -> Result<GridLayout, GridLayoutError> {
    validate_origin(origin)?;
    validate_available_size(available_size)?;
    validate_tracks(columns, GridAxis::Column)?;
    validate_tracks(rows, GridAxis::Row)?;
    validate_gap(column_gap, GridAxis::Column)?;
    validate_gap(row_gap, GridAxis::Row)?;

    let column_offsets = track_offsets(columns, column_gap)?;
    let row_offsets = track_offsets(rows, row_gap)?;
    let content_width = track_extent(columns, column_gap)?;
    let content_height = track_extent(rows, row_gap)?;

    let mut placements = Vec::with_capacity(items.len());
    for item in items {
        validate_concrete_item(*item, columns.len(), rows.len())?;
        let row_end = item.row_start + item.row_span;
        let column_end = item.column_start + item.column_span;

        let x = finite_add(origin.x, column_offsets[item.column_start])?;
        let y = finite_add(origin.y, row_offsets[item.row_start])?;
        let width = span_extent(&columns[item.column_start..column_end], column_gap)?;
        let height = span_extent(&rows[item.row_start..row_end], row_gap)?;
        placements.push(GridPlacement {
            node: item.node,
            area: Rect::new(x, y, width, height),
        });
    }

    Ok(GridLayout {
        items: placements,
        content_size: Size {
            width: content_width,
            height: content_height,
        },
        overflows_columns_axis: content_width > available_size.width,
        overflows_rows_axis: content_height > available_size.height,
    })
}

pub fn layout_fixed_grid_with_auto_placement(
    origin: Point,
    available_size: Size,
    columns: &[GridTrack],
    rows: &[GridTrack],
    column_gap: f32,
    row_gap: f32,
    requests: &[GridPlacementRequest],
) -> Result<GridLayout, GridLayoutError> {
    layout_fixed_grid(
        origin,
        available_size,
        columns,
        rows,
        column_gap,
        row_gap,
        &[],
    )?;

    let items = resolve_grid_placements(columns.len(), rows.len(), requests)?;
    layout_fixed_grid(
        origin,
        available_size,
        columns,
        rows,
        column_gap,
        row_gap,
        &items,
    )
}

pub fn resolve_grid_placements(
    column_count: usize,
    row_count: usize,
    requests: &[GridPlacementRequest],
) -> Result<Vec<GridItem>, GridLayoutError> {
    let mut resolved = vec![None; requests.len()];
    let mut occupied = Vec::new();

    for (index, request) in requests.iter().copied().enumerate() {
        validate_request_span(request)?;
        if let (Some(row_start), Some(column_start)) = (request.row_start, request.column_start) {
            let item = GridItem::new(request.node, row_start, column_start)
                .with_span(request.row_span, request.column_span);
            validate_concrete_item(item, column_count, row_count)?;
            resolved[index] = Some(item);
            occupied.push(item);
        }
    }

    for (index, request) in requests.iter().copied().enumerate() {
        if resolved[index].is_some() {
            continue;
        }
        let Some(item) = find_auto_placement(request, column_count, row_count, &occupied) else {
            return Err(GridLayoutError::AutoPlacementUnavailable { node: request.node });
        };
        resolved[index] = Some(item);
        occupied.push(item);
    }

    Ok(resolved
        .into_iter()
        .map(|item| item.expect("every grid placement request was resolved"))
        .collect())
}

pub fn resolve_content_sized_tracks(
    sizing: &[GridTrackSizing],
    axis: GridAxis,
    items: &[GridItem],
    contributions: &[GridItemContribution],
) -> Result<Vec<GridTrack>, GridLayoutError> {
    let mut states = initialize_track_sizing_states(sizing, axis)?;
    let mut planned_contributions = Vec::new();

    for item in items.iter().copied() {
        let (start, span) = match axis {
            GridAxis::Column => (item.column_start, item.column_span),
            GridAxis::Row => (item.row_start, item.row_span),
        };
        let end = start
            .checked_add(span)
            .ok_or(GridLayoutError::PlacementOutsideGrid { node: item.node })?;
        let Some(track_slice) = sizing.get(start..end) else {
            return Err(GridLayoutError::PlacementOutsideGrid { node: item.node });
        };
        if !track_slice
            .iter()
            .any(|track| matches!(track, GridTrackSizing::Auto))
        {
            continue;
        }
        if span != 1 {
            return Err(GridLayoutError::UnsupportedIntrinsicSpan {
                node: item.node,
                axis,
            });
        }
        let contribution = contributions
            .iter()
            .copied()
            .find(|contribution| contribution.node == item.node)
            .ok_or(GridLayoutError::MissingContribution {
                node: item.node,
                axis,
            })?;
        let size = GridIntrinsicContributions::from_legacy(contribution)
            .size_for(axis, GridIntrinsicContributionKind::MaxContent)?;
        planned_contributions.push(GridSpanningSizeContribution::new(
            item.node, start, span, size,
        ));
    }

    let planned =
        plan_auto_track_base_size_increases(&states, sizing, 0.0, axis, &planned_contributions)?;
    apply_planned_base_size_increases(&mut states, &planned)?;

    Ok(states
        .into_iter()
        .map(GridTrackSizingState::resolved_track)
        .collect())
}

fn initialize_track_sizing_states(
    sizing: &[GridTrackSizing],
    axis: GridAxis,
) -> Result<Vec<GridTrackSizingState>, GridLayoutError> {
    sizing
        .iter()
        .copied()
        .enumerate()
        .map(|(index, track)| match track {
            GridTrackSizing::Fixed(size) if size.is_finite() && size >= 0.0 => {
                Ok(GridTrackSizingState::fixed(size))
            }
            GridTrackSizing::Auto => Ok(GridTrackSizingState::intrinsic()),
            GridTrackSizing::Fixed(_) => Err(GridLayoutError::InvalidTrackSize { axis, index }),
        })
        .collect()
}

pub(crate) fn plan_auto_track_base_size_increases(
    states: &[GridTrackSizingState],
    sizing: &[GridTrackSizing],
    gap: f32,
    axis: GridAxis,
    contributions: &[GridSpanningSizeContribution],
) -> Result<Vec<f32>, GridLayoutError> {
    if states.len() != sizing.len() {
        return Err(GridLayoutError::GeometryOverflow);
    }
    validate_gap(gap, axis)?;

    let mut planned = vec![0.0_f32; states.len()];
    for contribution in contributions.iter().copied() {
        if contribution.span == 0 || !contribution.size.is_finite() || contribution.size < 0.0 {
            return Err(GridLayoutError::InvalidContribution {
                node: contribution.node,
                axis,
            });
        }
        let end = contribution.start.checked_add(contribution.span).ok_or(
            GridLayoutError::PlacementOutsideGrid {
                node: contribution.node,
            },
        )?;
        let Some(span_states) = states.get(contribution.start..end) else {
            return Err(GridLayoutError::PlacementOutsideGrid {
                node: contribution.node,
            });
        };
        let Some(span_sizing) = sizing.get(contribution.start..end) else {
            return Err(GridLayoutError::PlacementOutsideGrid {
                node: contribution.node,
            });
        };

        let affected = span_sizing
            .iter()
            .enumerate()
            .filter_map(|(offset, track)| {
                matches!(track, GridTrackSizing::Auto).then_some(contribution.start + offset)
            })
            .collect::<Vec<_>>();
        if affected.is_empty() {
            continue;
        }

        let mut occupied = 0.0;
        for (offset, state) in span_states.iter().enumerate() {
            occupied = finite_add(occupied, state.base_size)?;
            if offset + 1 < span_states.len() {
                occupied = finite_add(occupied, gap)?;
            }
        }
        let space = (contribution.size - occupied).max(0.0);
        if space == 0.0 {
            continue;
        }

        let incurred = distribute_base_size_space(states, &affected, space)?;
        for (index, increase) in incurred {
            planned[index] = planned[index].max(increase);
        }
    }

    Ok(planned)
}

pub(crate) fn apply_planned_base_size_increases(
    states: &mut [GridTrackSizingState],
    planned: &[f32],
) -> Result<(), GridLayoutError> {
    if states.len() != planned.len() {
        return Err(GridLayoutError::GeometryOverflow);
    }
    for (state, increase) in states.iter_mut().zip(planned.iter().copied()) {
        if !increase.is_finite() || increase < 0.0 {
            return Err(GridLayoutError::GeometryOverflow);
        }
        state.grow_base_to(finite_add(state.base_size, increase)?);
    }
    Ok(())
}

fn distribute_base_size_space(
    states: &[GridTrackSizingState],
    affected: &[usize],
    mut space: f32,
) -> Result<Vec<(usize, f32)>, GridLayoutError> {
    let mut increases = affected
        .iter()
        .copied()
        .map(|index| (index, 0.0))
        .collect::<Vec<_>>();
    let mut active = (0..increases.len()).collect::<Vec<_>>();

    while space > 0.0 && !active.is_empty() {
        active.retain(|slot| {
            let (track_index, increase) = increases[*slot];
            match states[track_index].growth_limit {
                GridTrackGrowthLimit::Finite(limit) => {
                    limit > states[track_index].base_size + increase
                }
                GridTrackGrowthLimit::Infinite => true,
            }
        });
        if active.is_empty() {
            break;
        }

        let share = space / active.len() as f32;
        if !share.is_finite() {
            return Err(GridLayoutError::GeometryOverflow);
        }

        let mut step = share;
        for slot in active.iter().copied() {
            let (track_index, increase) = increases[slot];
            if let GridTrackGrowthLimit::Finite(limit) = states[track_index].growth_limit {
                let remaining = (limit - states[track_index].base_size - increase).max(0.0);
                step = step.min(remaining);
            }
        }

        if step == 0.0 {
            break;
        }

        for slot in active.iter().copied() {
            increases[slot].1 = finite_add(increases[slot].1, step)?;
        }
        space = (space - step * active.len() as f32).max(0.0);

        if step == share {
            break;
        }
    }

    Ok(increases)
}

fn validate_request_span(request: GridPlacementRequest) -> Result<(), GridLayoutError> {
    if request.row_span == 0 || request.column_span == 0 {
        Err(GridLayoutError::InvalidSpan { node: request.node })
    } else {
        Ok(())
    }
}

fn validate_concrete_item(
    item: GridItem,
    column_count: usize,
    row_count: usize,
) -> Result<(), GridLayoutError> {
    if item.row_span == 0 || item.column_span == 0 {
        return Err(GridLayoutError::InvalidSpan { node: item.node });
    }
    let row_end = item
        .row_start
        .checked_add(item.row_span)
        .ok_or(GridLayoutError::PlacementOutsideGrid { node: item.node })?;
    let column_end = item
        .column_start
        .checked_add(item.column_span)
        .ok_or(GridLayoutError::PlacementOutsideGrid { node: item.node })?;
    if row_end > row_count
        || column_end > column_count
        || item.row_start >= row_count
        || item.column_start >= column_count
    {
        Err(GridLayoutError::PlacementOutsideGrid { node: item.node })
    } else {
        Ok(())
    }
}

fn find_auto_placement(
    request: GridPlacementRequest,
    column_count: usize,
    row_count: usize,
    occupied: &[GridItem],
) -> Option<GridItem> {
    match (request.row_start, request.column_start) {
        (Some(row), None) => {
            for column in 0..column_count {
                let candidate = GridItem::new(request.node, row, column)
                    .with_span(request.row_span, request.column_span);
                if candidate_fits(candidate, column_count, row_count, occupied) {
                    return Some(candidate);
                }
            }
        }
        (None, Some(column)) => {
            for row in 0..row_count {
                let candidate = GridItem::new(request.node, row, column)
                    .with_span(request.row_span, request.column_span);
                if candidate_fits(candidate, column_count, row_count, occupied) {
                    return Some(candidate);
                }
            }
        }
        (None, None) => {
            for row in 0..row_count {
                for column in 0..column_count {
                    let candidate = GridItem::new(request.node, row, column)
                        .with_span(request.row_span, request.column_span);
                    if candidate_fits(candidate, column_count, row_count, occupied) {
                        return Some(candidate);
                    }
                }
            }
        }
        (Some(_), Some(_)) => unreachable!("fully explicit requests resolve before auto-placement"),
    }
    None
}

fn candidate_fits(
    candidate: GridItem,
    column_count: usize,
    row_count: usize,
    occupied: &[GridItem],
) -> bool {
    if validate_concrete_item(candidate, column_count, row_count).is_err() {
        return false;
    }
    !occupied
        .iter()
        .copied()
        .any(|item| grid_items_overlap(candidate, item))
}

fn grid_items_overlap(left: GridItem, right: GridItem) -> bool {
    let left_row_end = left.row_start + left.row_span;
    let left_column_end = left.column_start + left.column_span;
    let right_row_end = right.row_start + right.row_span;
    let right_column_end = right.column_start + right.column_span;

    left.row_start < right_row_end
        && right.row_start < left_row_end
        && left.column_start < right_column_end
        && right.column_start < left_column_end
}

fn validate_origin(origin: Point) -> Result<(), GridLayoutError> {
    if origin.x.is_finite() && origin.y.is_finite() {
        Ok(())
    } else {
        Err(GridLayoutError::InvalidOrigin)
    }
}

fn validate_available_size(size: Size) -> Result<(), GridLayoutError> {
    if size.width.is_finite() && size.height.is_finite() && size.width >= 0.0 && size.height >= 0.0
    {
        Ok(())
    } else {
        Err(GridLayoutError::InvalidAvailableSize)
    }
}

fn validate_tracks(tracks: &[GridTrack], axis: GridAxis) -> Result<(), GridLayoutError> {
    for (index, track) in tracks.iter().enumerate() {
        if !track.base_size.is_finite() || track.base_size < 0.0 {
            return Err(GridLayoutError::InvalidTrackSize { axis, index });
        }
    }
    Ok(())
}

fn validate_gap(gap: f32, axis: GridAxis) -> Result<(), GridLayoutError> {
    if gap.is_finite() && gap >= 0.0 {
        Ok(())
    } else {
        Err(GridLayoutError::InvalidGap { axis })
    }
}

fn track_offsets(tracks: &[GridTrack], gap: f32) -> Result<Vec<f32>, GridLayoutError> {
    let mut offsets = Vec::with_capacity(tracks.len());
    let mut offset = 0.0_f32;
    for (index, track) in tracks.iter().enumerate() {
        offsets.push(offset);
        offset = finite_add(offset, track.base_size)?;
        if index + 1 < tracks.len() {
            offset = finite_add(offset, gap)?;
        }
    }
    Ok(offsets)
}

fn track_extent(tracks: &[GridTrack], gap: f32) -> Result<f32, GridLayoutError> {
    span_extent(tracks, gap)
}

fn span_extent(tracks: &[GridTrack], gap: f32) -> Result<f32, GridLayoutError> {
    let mut extent = 0.0_f32;
    for (index, track) in tracks.iter().enumerate() {
        extent = finite_add(extent, track.base_size)?;
        if index + 1 < tracks.len() {
            extent = finite_add(extent, gap)?;
        }
    }
    Ok(extent)
}

fn finite_add(left: f32, right: f32) -> Result<f32, GridLayoutError> {
    let value = left + right;
    value
        .is_finite()
        .then_some(value)
        .ok_or(GridLayoutError::GeometryOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(size: f32) -> GridTrack {
        GridTrack::new(size)
    }

    #[test]
    fn fixed_grid_places_items_in_explicit_tracks_with_gaps() {
        let layout = layout_fixed_grid(
            Point { x: 5.0, y: 7.0 },
            Size {
                width: 200.0,
                height: 200.0,
            },
            &[track(40.0), track(60.0)],
            &[track(20.0), track(30.0)],
            10.0,
            5.0,
            &[
                GridItem::new(LayoutNodeId(1), 0, 0),
                GridItem::new(LayoutNodeId(2), 1, 1),
            ],
        )
        .unwrap();

        assert_eq!(layout.items[0].area, Rect::new(5.0, 7.0, 40.0, 20.0));
        assert_eq!(layout.items[1].area, Rect::new(55.0, 32.0, 60.0, 30.0));
        assert_eq!(
            layout.content_size,
            Size {
                width: 110.0,
                height: 55.0,
            }
        );
        assert!(!layout.overflows_columns_axis);
        assert!(!layout.overflows_rows_axis);
    }

    #[test]
    fn spanning_item_area_includes_internal_track_gaps() {
        let layout = layout_fixed_grid(
            Point::default(),
            Size {
                width: 200.0,
                height: 200.0,
            },
            &[track(20.0), track(30.0), track(40.0)],
            &[track(10.0), track(15.0)],
            4.0,
            6.0,
            &[GridItem::new(LayoutNodeId(1), 0, 0).with_span(2, 2)],
        )
        .unwrap();

        assert_eq!(layout.items[0].area, Rect::new(0.0, 0.0, 54.0, 31.0));
    }

    #[test]
    fn overlapping_items_preserve_source_order() {
        let layout = layout_fixed_grid(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[track(50.0)],
            &[track(40.0)],
            0.0,
            0.0,
            &[
                GridItem::new(LayoutNodeId(7), 0, 0),
                GridItem::new(LayoutNodeId(3), 0, 0),
            ],
        )
        .unwrap();

        assert_eq!(layout.items[0].node, LayoutNodeId(7));
        assert_eq!(layout.items[1].node, LayoutNodeId(3));
        assert_eq!(layout.items[0].area, layout.items[1].area);
    }

    #[test]
    fn auto_placement_fills_explicit_grid_in_row_major_source_order() {
        let layout = layout_fixed_grid_with_auto_placement(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[track(20.0), track(30.0)],
            &[track(10.0), track(15.0)],
            0.0,
            0.0,
            &[
                GridPlacementRequest::auto(LayoutNodeId(1)),
                GridPlacementRequest::auto(LayoutNodeId(2)),
                GridPlacementRequest::auto(LayoutNodeId(3)),
            ],
        )
        .unwrap();

        assert_eq!(layout.items[0].area, Rect::new(0.0, 0.0, 20.0, 10.0));
        assert_eq!(layout.items[1].area, Rect::new(20.0, 0.0, 30.0, 10.0));
        assert_eq!(layout.items[2].area, Rect::new(0.0, 10.0, 20.0, 15.0));
    }

    #[test]
    fn explicit_placements_reserve_cells_before_auto_items() {
        let layout = layout_fixed_grid_with_auto_placement(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[track(20.0), track(30.0)],
            &[track(10.0), track(15.0)],
            0.0,
            0.0,
            &[
                GridPlacementRequest::auto(LayoutNodeId(1)),
                GridPlacementRequest::explicit(LayoutNodeId(2), 0, 0),
                GridPlacementRequest::auto(LayoutNodeId(3)),
            ],
        )
        .unwrap();

        assert_eq!(layout.items[0].area, Rect::new(20.0, 0.0, 30.0, 10.0));
        assert_eq!(layout.items[1].area, Rect::new(0.0, 0.0, 20.0, 10.0));
        assert_eq!(layout.items[2].area, Rect::new(0.0, 10.0, 20.0, 15.0));
        assert_eq!(layout.items[0].node, LayoutNodeId(1));
        assert_eq!(layout.items[1].node, LayoutNodeId(2));
        assert_eq!(layout.items[2].node, LayoutNodeId(3));
    }

    #[test]
    fn auto_placement_honors_spans_and_partially_explicit_axes() {
        let layout = layout_fixed_grid_with_auto_placement(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[track(10.0), track(20.0), track(30.0)],
            &[track(15.0), track(25.0)],
            0.0,
            0.0,
            &[
                GridPlacementRequest::explicit(LayoutNodeId(1), 0, 0),
                GridPlacementRequest::auto(LayoutNodeId(2)).with_span(1, 2),
                GridPlacementRequest::auto(LayoutNodeId(3)).with_row_start(Some(1)),
                GridPlacementRequest::auto(LayoutNodeId(4)).with_column_start(Some(2)),
            ],
        )
        .unwrap();

        assert_eq!(layout.items[1].area, Rect::new(10.0, 0.0, 50.0, 15.0));
        assert_eq!(layout.items[2].area, Rect::new(0.0, 15.0, 10.0, 25.0));
        assert_eq!(layout.items[3].area, Rect::new(30.0, 15.0, 30.0, 25.0));
    }

    #[test]
    fn explicit_overlap_is_allowed_but_blocks_auto_placement() {
        let layout = layout_fixed_grid_with_auto_placement(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[track(20.0), track(20.0)],
            &[track(20.0)],
            0.0,
            0.0,
            &[
                GridPlacementRequest::explicit(LayoutNodeId(1), 0, 0),
                GridPlacementRequest::explicit(LayoutNodeId(2), 0, 0),
                GridPlacementRequest::auto(LayoutNodeId(3)),
            ],
        )
        .unwrap();

        assert_eq!(layout.items[0].area, layout.items[1].area);
        assert_eq!(layout.items[2].area, Rect::new(20.0, 0.0, 20.0, 20.0));
    }

    #[test]
    fn auto_placement_fails_when_explicit_tracks_have_no_fitting_area() {
        assert_eq!(
            layout_fixed_grid_with_auto_placement(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 100.0,
                },
                &[track(20.0)],
                &[track(20.0)],
                0.0,
                0.0,
                &[
                    GridPlacementRequest::explicit(LayoutNodeId(1), 0, 0),
                    GridPlacementRequest::auto(LayoutNodeId(2)),
                ],
            ),
            Err(GridLayoutError::AutoPlacementUnavailable {
                node: LayoutNodeId(2),
            })
        );
    }

    #[test]
    fn invalid_tracks_and_gaps_are_rejected() {
        assert_eq!(
            layout_fixed_grid(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 100.0,
                },
                &[track(f32::NAN)],
                &[track(10.0)],
                0.0,
                0.0,
                &[],
            ),
            Err(GridLayoutError::InvalidTrackSize {
                axis: GridAxis::Column,
                index: 0,
            })
        );
        assert_eq!(
            layout_fixed_grid(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 100.0,
                },
                &[track(10.0)],
                &[track(10.0)],
                -1.0,
                0.0,
                &[],
            ),
            Err(GridLayoutError::InvalidGap {
                axis: GridAxis::Column,
            })
        );
    }

    #[test]
    fn zero_spans_and_out_of_bounds_placements_are_rejected() {
        let columns = [track(20.0)];
        let rows = [track(20.0)];
        assert_eq!(
            layout_fixed_grid(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 100.0,
                },
                &columns,
                &rows,
                0.0,
                0.0,
                &[GridItem::new(LayoutNodeId(1), 0, 0).with_span(0, 1)],
            ),
            Err(GridLayoutError::InvalidSpan {
                node: LayoutNodeId(1),
            })
        );
        assert_eq!(
            layout_fixed_grid(
                Point::default(),
                Size {
                    width: 100.0,
                    height: 100.0,
                },
                &columns,
                &rows,
                0.0,
                0.0,
                &[GridItem::new(LayoutNodeId(2), 1, 0)],
            ),
            Err(GridLayoutError::PlacementOutsideGrid {
                node: LayoutNodeId(2),
            })
        );
    }

    #[test]
    fn track_sizing_state_initializes_base_sizes_and_growth_limits() {
        let states = initialize_track_sizing_states(
            &[GridTrackSizing::Fixed(24.0), GridTrackSizing::Auto],
            GridAxis::Column,
        )
        .unwrap();

        assert_eq!(
            states,
            vec![
                GridTrackSizingState {
                    base_size: 24.0,
                    growth_limit: GridTrackGrowthLimit::Finite(24.0),
                },
                GridTrackSizingState {
                    base_size: 0.0,
                    growth_limit: GridTrackGrowthLimit::Infinite,
                },
            ]
        );
    }

    #[test]
    fn track_sizing_state_growth_respects_finite_limit() {
        let mut fixed = GridTrackSizingState::fixed(20.0);
        fixed.grow_base_to(50.0);
        assert_eq!(fixed.base_size, 20.0);

        let mut intrinsic = GridTrackSizingState::intrinsic();
        intrinsic.grow_base_to(50.0);
        assert_eq!(intrinsic.base_size, 50.0);
    }

    #[test]
    fn legacy_contribution_adapter_preserves_existing_scalar_sizes() {
        let contributions = GridIntrinsicContributions::from_legacy(GridItemContribution::new(
            LayoutNodeId(3),
            42.0,
            18.0,
        ));

        for kind in [
            GridIntrinsicContributionKind::Minimum,
            GridIntrinsicContributionKind::MinContent,
            GridIntrinsicContributionKind::MaxContent,
        ] {
            assert_eq!(
                contributions.size_for(GridAxis::Column, kind).unwrap(),
                42.0
            );
            assert_eq!(contributions.size_for(GridAxis::Row, kind).unwrap(), 18.0);
        }
    }

    #[test]
    fn intrinsic_contribution_kinds_are_selected_per_axis() {
        let contributions = GridIntrinsicContributions::new(
            LayoutNodeId(1),
            GridAxisIntrinsicContributions::new(12.0, 20.0, 36.0),
            GridAxisIntrinsicContributions::new(8.0, 14.0, 22.0),
        );

        assert_eq!(
            contributions
                .size_for(GridAxis::Column, GridIntrinsicContributionKind::Minimum)
                .unwrap(),
            12.0
        );
        assert_eq!(
            contributions
                .size_for(GridAxis::Column, GridIntrinsicContributionKind::MinContent)
                .unwrap(),
            20.0
        );
        assert_eq!(
            contributions
                .size_for(GridAxis::Column, GridIntrinsicContributionKind::MaxContent)
                .unwrap(),
            36.0
        );
        assert_eq!(
            contributions
                .size_for(GridAxis::Row, GridIntrinsicContributionKind::Minimum)
                .unwrap(),
            8.0
        );
        assert_eq!(
            contributions
                .size_for(GridAxis::Row, GridIntrinsicContributionKind::MaxContent)
                .unwrap(),
            22.0
        );
    }

    #[test]
    fn intrinsic_contribution_kinds_validate_selected_axis_geometry() {
        let contributions = GridIntrinsicContributions::new(
            LayoutNodeId(7),
            GridAxisIntrinsicContributions::new(10.0, f32::NAN, 20.0),
            GridAxisIntrinsicContributions::new(5.0, 6.0, 7.0),
        );

        assert_eq!(
            contributions.size_for(GridAxis::Column, GridIntrinsicContributionKind::MinContent,),
            Err(GridLayoutError::InvalidContribution {
                node: LayoutNodeId(7),
                axis: GridAxis::Column,
            })
        );
        assert_eq!(
            contributions
                .size_for(GridAxis::Row, GridIntrinsicContributionKind::MinContent)
                .unwrap(),
            6.0
        );
    }

    #[test]
    fn intrinsic_contribution_kinds_require_monotonic_sizes() {
        let contributions = GridIntrinsicContributions::new(
            LayoutNodeId(9),
            GridAxisIntrinsicContributions::new(30.0, 20.0, 25.0),
            GridAxisIntrinsicContributions::new(1.0, 1.0, 1.0),
        );

        assert_eq!(
            contributions.size_for(GridAxis::Column, GridIntrinsicContributionKind::Minimum,),
            Err(GridLayoutError::InvalidContribution {
                node: LayoutNodeId(9),
                axis: GridAxis::Column,
            })
        );
        assert_eq!(
            contributions
                .size_for(GridAxis::Row, GridIntrinsicContributionKind::MaxContent)
                .unwrap(),
            1.0
        );
    }

    #[test]
    fn spanning_distribution_plans_increases_without_mutating_state() {
        let states = initialize_track_sizing_states(
            &[
                GridTrackSizing::Auto,
                GridTrackSizing::Fixed(20.0),
                GridTrackSizing::Auto,
            ],
            GridAxis::Column,
        )
        .unwrap();
        let planned = plan_auto_track_base_size_increases(
            &states,
            &[
                GridTrackSizing::Auto,
                GridTrackSizing::Fixed(20.0),
                GridTrackSizing::Auto,
            ],
            5.0,
            GridAxis::Column,
            &[GridSpanningSizeContribution::new(
                LayoutNodeId(1),
                0,
                3,
                70.0,
            )],
        )
        .unwrap();

        assert_eq!(planned, vec![20.0, 0.0, 20.0]);
        assert_eq!(states[0].base_size, 0.0);
        assert_eq!(states[2].base_size, 0.0);
    }

    #[test]
    fn spanning_distribution_is_order_independent_across_items() {
        let states = initialize_track_sizing_states(
            &[GridTrackSizing::Auto, GridTrackSizing::Auto],
            GridAxis::Column,
        )
        .unwrap();
        let first = [
            GridSpanningSizeContribution::new(LayoutNodeId(1), 0, 2, 40.0),
            GridSpanningSizeContribution::new(LayoutNodeId(2), 0, 2, 60.0),
        ];
        let second = [first[1], first[0]];

        let planned_first = plan_auto_track_base_size_increases(
            &states,
            &[GridTrackSizing::Auto, GridTrackSizing::Auto],
            0.0,
            GridAxis::Column,
            &first,
        )
        .unwrap();
        let planned_second = plan_auto_track_base_size_increases(
            &states,
            &[GridTrackSizing::Auto, GridTrackSizing::Auto],
            0.0,
            GridAxis::Column,
            &second,
        )
        .unwrap();

        assert_eq!(planned_first, vec![30.0, 30.0]);
        assert_eq!(planned_second, planned_first);
    }

    #[test]
    fn planned_increases_apply_after_the_distribution_round() {
        let mut states = initialize_track_sizing_states(
            &[GridTrackSizing::Auto, GridTrackSizing::Auto],
            GridAxis::Column,
        )
        .unwrap();
        let planned = plan_auto_track_base_size_increases(
            &states,
            &[GridTrackSizing::Auto, GridTrackSizing::Auto],
            4.0,
            GridAxis::Column,
            &[GridSpanningSizeContribution::new(
                LayoutNodeId(1),
                0,
                2,
                44.0,
            )],
        )
        .unwrap();

        apply_planned_base_size_increases(&mut states, &planned).unwrap();

        assert_eq!(states[0].base_size, 20.0);
        assert_eq!(states[1].base_size, 20.0);
    }

    #[test]
    fn spanning_distribution_terminates_when_equal_share_underflows() {
        let states =
            initialize_track_sizing_states(&[GridTrackSizing::Auto; 8], GridAxis::Column).unwrap();
        let planned = plan_auto_track_base_size_increases(
            &states,
            &[GridTrackSizing::Auto; 8],
            0.0,
            GridAxis::Column,
            &[GridSpanningSizeContribution::new(
                LayoutNodeId(1),
                0,
                8,
                f32::from_bits(1),
            )],
        )
        .unwrap();

        assert_eq!(planned, vec![0.0; 8]);
    }

    #[test]
    fn spanning_distribution_respects_finite_growth_limits() {
        let states = [
            GridTrackSizingState {
                base_size: 10.0,
                growth_limit: GridTrackGrowthLimit::Finite(15.0),
            },
            GridTrackSizingState::intrinsic(),
        ];
        let planned = plan_auto_track_base_size_increases(
            &states,
            &[GridTrackSizing::Auto, GridTrackSizing::Auto],
            0.0,
            GridAxis::Column,
            &[GridSpanningSizeContribution::new(
                LayoutNodeId(1),
                0,
                2,
                40.0,
            )],
        )
        .unwrap();

        assert_eq!(planned, vec![5.0, 25.0]);
    }

    #[test]
    fn auto_tracks_use_largest_single_span_intrinsic_contribution() {
        let items = [
            GridItem::new(LayoutNodeId(1), 0, 0),
            GridItem::new(LayoutNodeId(2), 0, 0),
            GridItem::new(LayoutNodeId(3), 0, 1),
        ];
        let contributions = [
            GridItemContribution::new(LayoutNodeId(1), 30.0, 12.0),
            GridItemContribution::new(LayoutNodeId(2), 45.0, 18.0),
            GridItemContribution::new(LayoutNodeId(3), 80.0, 20.0),
        ];

        let columns = resolve_content_sized_tracks(
            &[GridTrackSizing::Auto, GridTrackSizing::Fixed(20.0)],
            GridAxis::Column,
            &items,
            &contributions,
        )
        .unwrap();
        let rows = resolve_content_sized_tracks(
            &[GridTrackSizing::Auto],
            GridAxis::Row,
            &items,
            &contributions,
        )
        .unwrap();

        assert_eq!(columns, vec![track(45.0), track(20.0)]);
        assert_eq!(rows, vec![track(20.0)]);
    }

    #[test]
    fn spanning_intrinsic_track_contribution_fails_closed() {
        let item = GridItem::new(LayoutNodeId(1), 0, 0).with_span(1, 2);
        assert_eq!(
            resolve_content_sized_tracks(
                &[GridTrackSizing::Auto, GridTrackSizing::Fixed(20.0)],
                GridAxis::Column,
                &[item],
                &[GridItemContribution::new(LayoutNodeId(1), 50.0, 10.0)],
            ),
            Err(GridLayoutError::UnsupportedIntrinsicSpan {
                node: LayoutNodeId(1),
                axis: GridAxis::Column,
            })
        );
    }

    #[test]
    fn explicit_grid_reports_track_overflow_against_available_size() {
        let layout = layout_fixed_grid(
            Point::default(),
            Size {
                width: 50.0,
                height: 30.0,
            },
            &[track(30.0), track(30.0)],
            &[track(20.0), track(20.0)],
            5.0,
            3.0,
            &[],
        )
        .unwrap();

        assert_eq!(layout.content_size.width, 65.0);
        assert_eq!(layout.content_size.height, 43.0);
        assert!(layout.overflows_columns_axis);
        assert!(layout.overflows_rows_axis);
    }

    #[test]
    fn empty_explicit_grid_has_zero_content_extent() {
        let layout = layout_fixed_grid(
            Point::default(),
            Size {
                width: 100.0,
                height: 100.0,
            },
            &[],
            &[],
            10.0,
            10.0,
            &[],
        )
        .unwrap();

        assert_eq!(layout.content_size, Size::default());
        assert!(layout.items.is_empty());
    }

    #[test]
    fn non_finite_accumulated_geometry_fails_closed() {
        assert_eq!(
            layout_fixed_grid(
                Point::default(),
                Size {
                    width: f32::MAX,
                    height: f32::MAX,
                },
                &[track(f32::MAX), track(f32::MAX)],
                &[track(1.0)],
                0.0,
                0.0,
                &[],
            ),
            Err(GridLayoutError::GeometryOverflow)
        );
    }
}
