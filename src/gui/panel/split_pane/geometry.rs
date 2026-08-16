use crate::gui::types::{Point, Rect, Vector2};

/// Axis along which a two-pane split is resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitPaneAxis {
    /// Resolve the first and second panes from left to right.
    #[default]
    Horizontal,
    /// Resolve the first and second panes from top to bottom.
    Vertical,
}

/// Pane selected by a runtime-owned split-pane collapse command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitPaneCollapsePolicy {
    /// Collapse the first, leading pane to its resolved minimum.
    FirstPane,
    /// Collapse the second, trailing pane to its resolved minimum.
    SecondPane,
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
        let ratio = sanitized_split_pane_ratio(parts.ratio);
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

/// Resolved collapse target using the same normalization and quantization as
/// the split layout engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitPaneCollapseTarget {
    pub(crate) ratio: f32,
    pub(crate) selected_extent: f32,
}

pub(crate) fn split_pane_collapse_target(
    parts: SplitPaneLayoutParts,
    policy: SplitPaneCollapsePolicy,
) -> Option<SplitPaneCollapseTarget> {
    let base = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        ratio: 0.5,
        ..parts
    });
    if !base.minima_satisfied {
        return None;
    }
    let total_extent = selected_extent(base.bounds, base.axis);
    let available_extent = total_extent - base.divider_extent;
    if !available_extent.is_finite() || available_extent <= 0.0 {
        return None;
    }

    let requested_ratio = match policy {
        SplitPaneCollapsePolicy::FirstPane => base.first_min_extent / available_extent,
        SplitPaneCollapsePolicy::SecondPane => {
            (available_extent - base.second_min_extent) / available_extent
        }
    };
    let ratio = sanitized_split_pane_ratio(requested_ratio);
    let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts { ratio, ..parts });
    let (first, _divider, second) = quantized_split_pane_rects(resolved);
    let selected_extent = match policy {
        SplitPaneCollapsePolicy::FirstPane => selected_extent(first, base.axis),
        SplitPaneCollapsePolicy::SecondPane => selected_extent(second, base.axis),
    };
    selected_extent
        .is_finite()
        .then_some(SplitPaneCollapseTarget {
            ratio,
            selected_extent,
        })
}

/// Quantize resolved split rectangles with the layout engine's cumulative
/// rounded-boundary contract.
pub(crate) fn quantized_split_pane_rects(resolved: SplitPaneLayout) -> (Rect, Rect, Rect) {
    let outer = round_rect(resolved.bounds);
    let total_extent = selected_extent(outer, resolved.axis).max(0.0);
    let divider_extent = if total_extent > 0.0 && resolved.divider_extent > 0.0 {
        resolved.divider_extent.round().max(1.0).min(total_extent)
    } else {
        0.0
    };
    let first_extent = selected_extent(resolved.first, resolved.axis)
        .round()
        .clamp(0.0, total_extent - divider_extent);
    let second_extent = total_extent - divider_extent - first_extent;

    let q0 = match resolved.axis {
        SplitPaneAxis::Horizontal => outer.min.x,
        SplitPaneAxis::Vertical => outer.min.y,
    };
    let q1 = q0 + first_extent;
    let q2 = q1 + divider_extent;
    let q3 = q2 + second_extent;

    (
        rect_for_axis_span(outer, resolved.axis, q0, q1),
        rect_for_axis_span(outer, resolved.axis, q1, q2),
        rect_for_axis_span(outer, resolved.axis, q2, q3),
    )
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

fn selected_extent(rect: Rect, axis: SplitPaneAxis) -> f32 {
    match axis {
        SplitPaneAxis::Horizontal => rect.width(),
        SplitPaneAxis::Vertical => rect.height(),
    }
}

fn rect_for_axis_span(outer: Rect, axis: SplitPaneAxis, start: f32, end: f32) -> Rect {
    match axis {
        SplitPaneAxis::Horizontal => {
            Rect::from_min_max(Point::new(start, outer.min.y), Point::new(end, outer.max.y))
        }
        SplitPaneAxis::Vertical => {
            Rect::from_min_max(Point::new(outer.min.x, start), Point::new(outer.max.x, end))
        }
    }
}

fn round_rect(rect: Rect) -> Rect {
    let min_x = rect.min.x.floor();
    let min_y = rect.min.y.floor();
    let width = rect.width().round().max(0.0);
    let height = rect.height().round().max(0.0);
    Rect::from_min_size(Point::new(min_x, min_y), Vector2::new(width, height))
}

/// Sanitize a split-pane ratio using the static geometry contract.
pub(crate) fn sanitized_split_pane_ratio(value: f32) -> f32 {
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
