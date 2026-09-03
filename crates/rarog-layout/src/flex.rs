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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexLayoutError {
    InvalidAvailableSize,
    InvalidOrigin,
    InvalidItemSize { node: LayoutNodeId },
    InvalidMargin { node: LayoutNodeId },
    NegativeMarginUnsupported { node: LayoutNodeId },
    GeometryOverflow,
}

impl fmt::Display for FlexLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAvailableSize => {
                formatter.write_str("flex available size must be finite and non-negative")
            }
            Self::InvalidOrigin => formatter.write_str("flex origin must be finite"),
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
            Self::GeometryOverflow => {
                formatter.write_str("flex layout geometry overflowed the finite coordinate range")
            }
        }
    }
}

impl std::error::Error for FlexLayoutError {}

pub fn layout_single_line_flex_row(
    origin: Point,
    available_size: Size,
    items: &[FlexRowItem],
) -> Result<FlexRowLayout, FlexLayoutError> {
    validate_origin(origin)?;
    validate_size(available_size).map_err(|_| FlexLayoutError::InvalidAvailableSize)?;

    let mut cursor_x = origin.x;
    let mut max_cross_size = 0.0_f32;
    let mut placements = Vec::with_capacity(items.len());

    for item in items {
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
        let cross_size = finite_add(item.margin.top, item.base_size.height)?;
        let cross_size = finite_add(cross_size, item.margin.bottom)?;
        max_cross_size = max_cross_size.max(cross_size);
    }

    let used_main_size = cursor_x - origin.x;
    if !used_main_size.is_finite() {
        return Err(FlexLayoutError::GeometryOverflow);
    }
    let used_main_size = used_main_size.max(0.0);
    Ok(FlexRowLayout {
        items: placements,
        content_size: Size {
            width: used_main_size,
            height: max_cross_size,
        },
        overflows_main_axis: used_main_size > available_size.width,
        overflows_cross_axis: max_cross_size > available_size.height,
    })
}

fn finite_add(left: f32, right: f32) -> Result<f32, FlexLayoutError> {
    let value = left + right;
    value
        .is_finite()
        .then_some(value)
        .ok_or(FlexLayoutError::GeometryOverflow)
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
