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
    InvalidGap { axis: GridAxis },
    InvalidSpan { node: LayoutNodeId },
    PlacementOutsideGrid { node: LayoutNodeId },
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
                write!(formatter, "grid {axis:?} track {index} must be finite and non-negative")
            }
            Self::InvalidGap { axis } => {
                write!(formatter, "grid {axis:?} gap must be finite and non-negative")
            }
            Self::InvalidSpan { node } => {
                write!(formatter, "grid item {node:?} must span at least one row and column")
            }
            Self::PlacementOutsideGrid { node } => {
                write!(formatter, "grid item {node:?} placement is outside the explicit grid")
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
        if row_end > rows.len()
            || column_end > columns.len()
            || item.row_start >= rows.len()
            || item.column_start >= columns.len()
        {
            return Err(GridLayoutError::PlacementOutsideGrid { node: item.node });
        }

        let x = finite_add(origin.x, column_offsets[item.column_start])?;
        let y = finite_add(origin.y, row_offsets[item.row_start])?;
        let width = span_extent(
            &columns[item.column_start..column_end],
            column_gap,
        )?;
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

fn validate_origin(origin: Point) -> Result<(), GridLayoutError> {
    if origin.x.is_finite() && origin.y.is_finite() {
        Ok(())
    } else {
        Err(GridLayoutError::InvalidOrigin)
    }
}

fn validate_available_size(size: Size) -> Result<(), GridLayoutError> {
    if size.width.is_finite()
        && size.height.is_finite()
        && size.width >= 0.0
        && size.height >= 0.0
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

        assert_eq!(
            layout.items[0].area,
            Rect::new(5.0, 7.0, 40.0, 20.0)
        );
        assert_eq!(
            layout.items[1].area,
            Rect::new(55.0, 32.0, 60.0, 30.0)
        );
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

        assert_eq!(
            layout.items[0].area,
            Rect::new(0.0, 0.0, 54.0, 31.0)
        );
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
