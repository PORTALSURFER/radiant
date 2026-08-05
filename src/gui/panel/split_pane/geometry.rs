use crate::gui::types::{Point, Rect};

/// Axis along which a two-pane split is resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitPaneAxis {
    /// Resolve the first and second panes from left to right.
    #[default]
    Horizontal,
    /// Resolve the first and second panes from top to bottom.
    Vertical,
}

/// Named geometry inputs for a resolved split-pane layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitPaneLayoutParts {
    /// Bounds containing the resolved panes and divider.
    pub bounds: Rect,
    /// Axis along which the panes are ordered.
    pub axis: SplitPaneAxis,
    /// Requested normalized extent of the first pane, clamped to `0.0..=1.0`.
    pub ratio: f32,
    /// Requested divider extent along the split axis.
    pub divider_extent: f32,
    /// Minimum extent requested for the first pane.
    pub first_min_extent: f32,
    /// Minimum extent requested for the second pane.
    pub second_min_extent: f32,
}

impl Default for SplitPaneLayoutParts {
    fn default() -> Self {
        Self {
            bounds: Rect::default(),
            axis: SplitPaneAxis::default(),
            ratio: 0.5,
            divider_extent: 0.0,
            first_min_extent: 0.0,
            second_min_extent: 0.0,
        }
    }
}

/// Resolved backend-neutral geometry for a two-pane split.
///
/// `first` and `second` are ordered left-to-right for a horizontal split and
/// top-to-bottom for a vertical split. The three rectangles are contiguous,
/// stay inside `bounds`, and share edges without overlapping with positive
/// area. A non-finite ratio resolves to a balanced `0.5` request, while
/// non-finite divider and minimum extents resolve to `0.0` before resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitPaneLayout {
    /// Finite, normalized bounds used for resolution.
    pub bounds: Rect,
    /// Axis along which the panes are ordered.
    pub axis: SplitPaneAxis,
    /// Sanitized requested normalized extent of the first pane.
    ///
    /// Minimum constraints can move the resolved divider away from this
    /// requested ratio; inspect the resolved rectangles for the effective
    /// ratio.
    pub ratio: f32,
    /// Resolved divider extent along the split axis.
    pub divider_extent: f32,
    /// Sanitized minimum extent requested for the first pane.
    pub first_min_extent: f32,
    /// Sanitized minimum extent requested for the second pane.
    pub second_min_extent: f32,
    /// Resolved first pane rectangle.
    pub first: Rect,
    /// Resolved divider rectangle.
    pub divider: Rect,
    /// Resolved second pane rectangle.
    pub second: Rect,
    /// Whether both requested pane minima were satisfied.
    ///
    /// This is `false` when the divider and both minima cannot fit in the
    /// available extent. In that case the panes retain the sanitized ratio
    /// over the remaining non-divider extent as a deterministic fallback.
    pub minima_satisfied: bool,
}

impl SplitPaneLayout {
    /// Resolve split-pane geometry from named inputs.
    pub fn from_parts(parts: SplitPaneLayoutParts) -> Self {
        let bounds = normalized_rect(parts.bounds);
        let ratio = sanitize_ratio(parts.ratio);
        let divider_extent = sanitize_nonnegative(parts.divider_extent);
        let first_min_extent = sanitize_nonnegative(parts.first_min_extent);
        let second_min_extent = sanitize_nonnegative(parts.second_min_extent);
        let total_extent = axis_extent(bounds, parts.axis);
        let requested_divider_extent = divider_extent.min(total_extent);
        let available_extent = total_extent - requested_divider_extent;
        let minima_fit = first_min_extent <= available_extent
            && second_min_extent <= available_extent - first_min_extent;
        let preferred_first_extent = available_extent * ratio;
        let first_extent = if minima_fit {
            preferred_first_extent.clamp(first_min_extent, available_extent - second_min_extent)
        } else {
            preferred_first_extent
        };
        let first_extent = sanitize_nonnegative(first_extent).min(available_extent);
        let (first, divider, second) =
            resolved_rects(bounds, parts.axis, first_extent, requested_divider_extent);
        let resolved_first_extent = axis_extent(first, parts.axis);
        let resolved_divider_extent = axis_extent(divider, parts.axis);
        let resolved_second_extent = axis_extent(second, parts.axis);

        Self {
            bounds,
            axis: parts.axis,
            ratio,
            divider_extent: resolved_divider_extent,
            first_min_extent,
            second_min_extent,
            first,
            divider,
            second,
            minima_satisfied: minima_fit
                && resolved_first_extent >= first_min_extent
                && resolved_second_extent >= second_min_extent,
        }
    }

    /// Resolve split-pane geometry from positional compatibility arguments.
    pub fn new(
        bounds: Rect,
        axis: SplitPaneAxis,
        ratio: f32,
        divider_extent: f32,
        first_min_extent: f32,
        second_min_extent: f32,
    ) -> Self {
        Self::from_parts(SplitPaneLayoutParts {
            bounds,
            axis,
            ratio,
            divider_extent,
            first_min_extent,
            second_min_extent,
        })
    }
}

impl Default for SplitPaneLayout {
    fn default() -> Self {
        Self::from_parts(SplitPaneLayoutParts::default())
    }
}

fn resolved_rects(
    bounds: Rect,
    axis: SplitPaneAxis,
    first_extent: f32,
    divider_extent: f32,
) -> (Rect, Rect, Rect) {
    let (axis_min, axis_max, cross_min, cross_max) = match axis {
        SplitPaneAxis::Horizontal => (bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y),
        SplitPaneAxis::Vertical => (bounds.min.y, bounds.max.y, bounds.min.x, bounds.max.x),
    };
    let divider_start = finite_or(axis_min + first_extent, axis_min).clamp(axis_min, axis_max);
    let divider_end =
        finite_or(divider_start + divider_extent, axis_max).clamp(divider_start, axis_max);

    match axis {
        SplitPaneAxis::Horizontal => (
            Rect::from_min_max(
                Point::new(axis_min, cross_min),
                Point::new(divider_start, cross_max),
            ),
            Rect::from_min_max(
                Point::new(divider_start, cross_min),
                Point::new(divider_end, cross_max),
            ),
            Rect::from_min_max(
                Point::new(divider_end, cross_min),
                Point::new(axis_max, cross_max),
            ),
        ),
        SplitPaneAxis::Vertical => (
            Rect::from_min_max(
                Point::new(cross_min, axis_min),
                Point::new(cross_max, divider_start),
            ),
            Rect::from_min_max(
                Point::new(cross_min, divider_start),
                Point::new(cross_max, divider_end),
            ),
            Rect::from_min_max(
                Point::new(cross_min, divider_end),
                Point::new(cross_max, axis_max),
            ),
        ),
    }
}

fn normalized_rect(rect: Rect) -> Rect {
    let (min_x, max_x) = normalized_axis(rect.min.x, rect.max.x);
    let (min_y, max_y) = normalized_axis(rect.min.y, rect.max.y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}

fn normalized_axis(start: f32, end: f32) -> (f32, f32) {
    let start = finite_or(start, 0.0);
    let end = finite_or(end, start);
    let (min, max) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    if (max - min).is_finite() {
        (min, max)
    } else {
        (min, min)
    }
}

fn axis_extent(rect: Rect, axis: SplitPaneAxis) -> f32 {
    let extent = match axis {
        SplitPaneAxis::Horizontal => rect.width(),
        SplitPaneAxis::Vertical => rect.height(),
    };
    if extent.is_finite() {
        extent.max(0.0)
    } else {
        0.0
    }
}

fn sanitize_ratio(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn sanitize_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
