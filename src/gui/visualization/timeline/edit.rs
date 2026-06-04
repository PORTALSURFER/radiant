use super::TimelineCoordinateMapper;
use crate::{
    gui::{
        range::{NormalizedRange, normalized_fraction_to_micros, normalized_fraction_to_milli},
        types::{Point, Rect, Rgba8},
        visualization::{SampledCurveStrokeParts, push_sampled_curve_stroke},
    },
    runtime::{PaintPrimitive, push_visible_fill_rect},
    widgets::WidgetId,
};

/// Editable range and fade handles for a normalized timeline or signal view.
///
/// The structure is deliberately host-neutral: it models a selected interval,
/// optional leading/trailing handle positions, and optional curve controls.
/// Hosts decide whether those controls represent animation ramps, trim previews,
/// easing handles, or other domain behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreview {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

/// Standard edit-preview handles for a normalized timeline or signal surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditHandle {
    /// Leading/top handle at the leading ramp end.
    LeadingEnd,
    /// Leading/bottom handle at the selected range start.
    LeadingStart,
    /// Leading outer handle before the selected range.
    LeadingOuterStart,
    /// Trailing/top handle at the trailing ramp start.
    TrailingStart,
    /// Trailing/bottom handle at the selected range end.
    TrailingEnd,
    /// Trailing outer handle after the selected range.
    TrailingOuterEnd,
}

impl TimelineEditHandle {
    /// Return the standard hit-test and paint order for timeline edit handles.
    ///
    /// Inner ramp handles are checked before selection-edge handles, and outer
    /// handles are checked last. This matches the default visual priority for
    /// compact timeline and signal editors while still letting hosts supply a
    /// custom order to [`TimelineEditPreview::handle_at`] when needed.
    pub const fn standard_order() -> [Self; 6] {
        [
            Self::LeadingEnd,
            Self::TrailingStart,
            Self::LeadingStart,
            Self::TrailingEnd,
            Self::LeadingOuterStart,
            Self::TrailingOuterEnd,
        ]
    }
}

/// Standard editable regions around a selected timeline interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditRegion {
    /// Leading region inside the selected interval.
    LeadingInner,
    /// Leading region before the selected interval.
    LeadingOuter,
    /// Trailing region inside the selected interval.
    TrailingInner,
    /// Trailing region after the selected interval.
    TrailingOuter,
}

/// Standard editable ramp side for a normalized timeline interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineEditRampSide {
    /// Leading ramp before or at the selected interval start.
    Leading,
    /// Trailing ramp at or after the selected interval end.
    Trailing,
}

impl TimelineEditRegion {
    /// Return the standard paint order for timeline edit regions.
    pub const fn standard_order() -> [Self; 4] {
        [
            Self::LeadingInner,
            Self::TrailingInner,
            Self::LeadingOuter,
            Self::TrailingOuter,
        ]
    }
}

/// Paint policy for standard timeline edit ramp curve strokes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineEditCurveStrokeParts {
    /// Widget that owns the generated stroke primitive.
    pub widget_id: WidgetId,
    /// Timeline-to-pixel mapper for the edited surface.
    pub mapper: TimelineCoordinateMapper,
    /// Stroke color used for all emitted ramp curves.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub stroke_width: f32,
    /// Desired logical pixels between samples before min/max clamping.
    pub pixels_per_step: f32,
    /// Minimum number of intervals used for each visible ramp curve.
    pub min_steps: usize,
    /// Maximum number of intervals used for each visible ramp curve.
    pub max_steps: usize,
}

impl TimelineEditCurveStrokeParts {
    /// Build standard timeline edit curve stroke parts.
    pub const fn new(
        widget_id: WidgetId,
        mapper: TimelineCoordinateMapper,
        color: Rgba8,
        stroke_width: f32,
    ) -> Self {
        Self {
            widget_id,
            mapper,
            color,
            stroke_width,
            pixels_per_step: 4.0,
            min_steps: 10,
            max_steps: 96,
        }
    }

    /// Override the desired logical pixels between sampled curve points.
    pub const fn pixels_per_step(mut self, pixels_per_step: f32) -> Self {
        self.pixels_per_step = pixels_per_step;
        self
    }

    /// Override the step-count clamp used after pixel-width projection.
    pub const fn step_bounds(mut self, min_steps: usize, max_steps: usize) -> Self {
        self.min_steps = min_steps;
        self.max_steps = max_steps;
        self
    }
}

/// Geometry policy for projecting edit-preview handles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineEditHandleGeometry {
    /// Horizontal and vertical bounds of the timeline or signal surface.
    pub bounds: Rect,
    /// Visible rectangle for the edited selection.
    pub selection_rect: Rect,
    /// Logical handle size in pixels.
    pub handle_size: f32,
}

impl TimelineEditHandleGeometry {
    /// Build handle projection geometry for a visible edit selection.
    pub const fn new(bounds: Rect, selection_rect: Rect, handle_size: f32) -> Self {
        Self {
            bounds,
            selection_rect,
            handle_size,
        }
    }

    /// Return the effective handle size after clamping to the surface bounds.
    pub fn clamped_handle_size(self) -> f32 {
        normalized_handle_size(self.bounds, self.handle_size)
    }
}

/// Geometry policy for projecting edit-preview regions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineEditRegionGeometry {
    /// Horizontal and vertical bounds of the timeline or signal surface.
    pub bounds: Rect,
    /// Visible rectangle for the edited selection.
    pub selection_rect: Rect,
}

impl TimelineEditRegionGeometry {
    /// Build region projection geometry for a visible edit selection.
    pub const fn new(bounds: Rect, selection_rect: Rect) -> Self {
        Self {
            bounds,
            selection_rect,
        }
    }
}

/// Named edit-preview parts for timeline handle projection.
///
/// Hosts can fill only the handles they need while keeping range, leading
/// handles, trailing handles, and curve controls readable at call sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreviewParts {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

/// Optional ramp projection for a normalized timeline edit preview.
///
/// A ramp is deliberately domain-neutral: it may represent an audio fade, an
/// animation easing segment, an opacity transition, a trim preview, or any
/// other leading/trailing edit affordance attached to a selected interval.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TimelineEditRamp {
    /// Ramp length as a fraction of the selected interval width.
    pub length_fraction: f32,
    /// Optional extension outside the selected interval as a fraction of the
    /// selected interval width.
    pub outer_fraction: f32,
    /// Optional curve/control value in normalized `0.0..=1.0` space.
    pub curve_fraction: Option<f32>,
}

impl TimelineEditRamp {
    /// Build a ramp from normalized length, outer extension, and optional curve.
    pub const fn new(
        length_fraction: f32,
        outer_fraction: f32,
        curve_fraction: Option<f32>,
    ) -> Self {
        Self {
            length_fraction,
            outer_fraction,
            curve_fraction,
        }
    }

    /// Build a ramp with no outer extension.
    pub const fn from_length(length_fraction: f32, curve_fraction: Option<f32>) -> Self {
        Self::new(length_fraction, 0.0, curve_fraction)
    }
}

impl TimelineEditPreview {
    /// Build an edit preview from named handle parts.
    pub fn from_parts(parts: TimelineEditPreviewParts) -> Self {
        Self {
            selection: parts.selection,
            leading_end_milli: parts.leading_end_milli,
            leading_end_micros: parts.leading_end_micros,
            leading_inner_start_milli: parts.leading_inner_start_milli,
            leading_inner_start_micros: parts.leading_inner_start_micros,
            leading_curve_milli: parts.leading_curve_milli,
            trailing_start_milli: parts.trailing_start_milli,
            trailing_start_micros: parts.trailing_start_micros,
            trailing_inner_end_milli: parts.trailing_inner_end_milli,
            trailing_inner_end_micros: parts.trailing_inner_end_micros,
            trailing_curve_milli: parts.trailing_curve_milli,
        }
    }

    /// Build an edit preview from a selected range and optional normalized ramps.
    ///
    /// The selected range supplies the durable timeline interval. Ramp lengths
    /// and outer extensions are fractions of that interval width, so hosts can
    /// project domain data into standard leading/trailing edit handles without
    /// duplicating milli/micro conversion and endpoint math.
    pub fn from_normalized_ramps(
        selection: NormalizedRange,
        leading: Option<TimelineEditRamp>,
        trailing: Option<TimelineEditRamp>,
    ) -> Self {
        let start = selection.start_fraction();
        let end = selection.end_fraction();
        let width = selection.width_fraction();
        Self::from_parts(TimelineEditPreviewParts {
            selection: Some(selection),
            leading_end_milli: leading
                .map(|ramp| normalized_fraction_to_milli(start + width * ramp.length_fraction)),
            leading_end_micros: leading
                .map(|ramp| normalized_fraction_to_micros(start + width * ramp.length_fraction)),
            leading_inner_start_milli: leading
                .map(|ramp| normalized_fraction_to_milli(start - width * ramp.outer_fraction)),
            leading_inner_start_micros: leading
                .map(|ramp| normalized_fraction_to_micros(start - width * ramp.outer_fraction)),
            leading_curve_milli: leading
                .and_then(|ramp| ramp.curve_fraction.map(normalized_fraction_to_milli)),
            trailing_start_milli: trailing
                .map(|ramp| normalized_fraction_to_milli(end - width * ramp.length_fraction)),
            trailing_start_micros: trailing
                .map(|ramp| normalized_fraction_to_micros(end - width * ramp.length_fraction)),
            trailing_inner_end_milli: trailing
                .map(|ramp| normalized_fraction_to_milli(end + width * ramp.outer_fraction)),
            trailing_inner_end_micros: trailing
                .map(|ramp| normalized_fraction_to_micros(end + width * ramp.outer_fraction)),
            trailing_curve_milli: trailing
                .and_then(|ramp| ramp.curve_fraction.map(normalized_fraction_to_milli)),
        })
    }

    /// Return the normalized micro-position for a standard edit handle.
    pub fn handle_micros(self, handle: TimelineEditHandle) -> Option<u32> {
        let selection = self.selection?;
        match handle {
            TimelineEditHandle::LeadingEnd => {
                Some(self.leading_end_micros.unwrap_or(selection.start_micros))
            }
            TimelineEditHandle::LeadingStart => {
                self.leading_end_micros.map(|_| selection.start_micros)
            }
            TimelineEditHandle::LeadingOuterStart => self.leading_end_micros.and(
                self.leading_inner_start_micros
                    .or(Some(selection.start_micros)),
            ),
            TimelineEditHandle::TrailingStart => {
                Some(self.trailing_start_micros.unwrap_or(selection.end_micros))
            }
            TimelineEditHandle::TrailingEnd => {
                self.trailing_start_micros.map(|_| selection.end_micros)
            }
            TimelineEditHandle::TrailingOuterEnd => self.trailing_start_micros.and(
                self.trailing_inner_end_micros
                    .or(Some(selection.end_micros)),
            ),
        }
    }

    /// Project the currently visible edit selection into the mapper rectangle.
    pub fn selection_rect(self, mapper: TimelineCoordinateMapper) -> Option<Rect> {
        let selection = self.selection?;
        let viewport = mapper.viewport;
        if selection.end_micros < viewport.start_micros
            || selection.start_micros > viewport.end_micros
        {
            return None;
        }
        let start_x = mapper.x_for_micros(selection.start_micros);
        let end_x = mapper.x_for_micros(selection.end_micros);
        if (end_x - start_x).abs() <= f32::EPSILON {
            return None;
        }
        Some(Rect::from_min_max(
            Point::new(start_x.min(end_x), mapper.rect.min.y),
            Point::new(start_x.max(end_x), mapper.rect.max.y),
        ))
    }

    /// Build standard handle geometry for the visible edit selection.
    pub fn handle_geometry(
        self,
        mapper: TimelineCoordinateMapper,
        handle_size: f32,
    ) -> Option<TimelineEditHandleGeometry> {
        let selection_rect = self.selection_rect(mapper)?;
        Some(TimelineEditHandleGeometry::new(
            mapper.rect,
            selection_rect,
            handle_size,
        ))
    }

    /// Build standard region geometry for the visible edit selection.
    pub fn region_geometry(
        self,
        mapper: TimelineCoordinateMapper,
    ) -> Option<TimelineEditRegionGeometry> {
        let selection_rect = self.selection_rect(mapper)?;
        Some(TimelineEditRegionGeometry::new(mapper.rect, selection_rect))
    }

    /// Project a standard edit handle into a hit-test or paint rectangle.
    pub fn handle_rect(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        handle: TimelineEditHandle,
    ) -> Option<Rect> {
        let micros = self.handle_micros(handle)?;
        if micros < mapper.viewport.start_micros || micros > mapper.viewport.end_micros {
            return None;
        }
        let size = geometry.clamped_handle_size();
        let x = mapper.x_for_micros(micros);
        let horizontal = geometry.bounds.vertical_strip_around_x(x, size);
        let vertical =
            edit_handle_vertical_band(geometry.bounds, geometry.selection_rect, handle, size);
        horizontal.intersection(vertical)
    }

    /// Project a standard edit-preview region into a paint rectangle.
    pub fn region_rect(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditRegionGeometry,
        region: TimelineEditRegion,
    ) -> Option<Rect> {
        let selection = self.selection?;
        match region {
            TimelineEditRegion::LeadingInner => {
                let end = self.leading_end_micros.unwrap_or(selection.start_micros);
                if end <= selection.start_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, end)?;
                let right_x = x.clamp(geometry.selection_rect.min.x, geometry.selection_rect.max.x);
                Some(
                    geometry
                        .selection_rect
                        .left_edge_strip(right_x - geometry.selection_rect.min.x),
                )
            }
            TimelineEditRegion::TrailingInner => {
                let start = self.trailing_start_micros.unwrap_or(selection.end_micros);
                if start >= selection.end_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, start)?;
                let left_x = x.clamp(geometry.selection_rect.min.x, geometry.selection_rect.max.x);
                Some(
                    geometry
                        .selection_rect
                        .right_edge_strip(geometry.selection_rect.max.x - left_x),
                )
            }
            TimelineEditRegion::LeadingOuter => {
                let start = self.leading_inner_start_micros?;
                if start >= selection.start_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, start)?;
                let left_x = x.clamp(geometry.bounds.min.x, geometry.selection_rect.min.x);
                let outer_bounds = Rect::from_min_max(
                    Point::new(geometry.bounds.min.x, geometry.selection_rect.min.y),
                    Point::new(geometry.selection_rect.min.x, geometry.selection_rect.max.y),
                );
                Some(outer_bounds.right_edge_strip(geometry.selection_rect.min.x - left_x))
            }
            TimelineEditRegion::TrailingOuter => {
                let end = self.trailing_inner_end_micros?;
                if end <= selection.end_micros {
                    return None;
                }
                let x = visible_x_for_micros(mapper, end)?;
                let right_x = x.clamp(geometry.selection_rect.max.x, geometry.bounds.max.x);
                let outer_bounds = Rect::from_min_max(
                    Point::new(geometry.selection_rect.max.x, geometry.selection_rect.min.y),
                    Point::new(geometry.bounds.max.x, geometry.selection_rect.max.y),
                );
                Some(outer_bounds.left_edge_strip(right_x - geometry.selection_rect.max.x))
            }
        }
    }

    /// Return visible rectangles for the standard edit-preview regions.
    pub fn standard_region_rects(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditRegionGeometry,
    ) -> impl Iterator<Item = (TimelineEditRegion, Rect)> {
        TimelineEditRegion::standard_order()
            .into_iter()
            .filter_map(move |region| {
                self.region_rect(mapper, geometry, region)
                    .map(|rect| (region, rect))
            })
    }

    /// Return visible rectangles for the standard edit-preview handles.
    pub fn standard_handle_rects(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
    ) -> impl Iterator<Item = (TimelineEditHandle, Rect)> {
        TimelineEditHandle::standard_order()
            .into_iter()
            .filter_map(move |handle| {
                self.handle_rect(mapper, geometry, handle)
                    .map(|rect| (handle, rect))
            })
    }

    /// Append guarded filled rectangles for the standard edit-preview regions.
    ///
    /// The caller owns region colors while Radiant owns projection, standard
    /// region order, and finite/visible paint emission.
    pub fn push_standard_region_fills(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditRegionGeometry,
        mut color_for_region: impl FnMut(TimelineEditRegion) -> Rgba8,
    ) {
        for (region, rect) in self.standard_region_rects(mapper, geometry) {
            push_visible_fill_rect(primitives, widget_id, rect, color_for_region(region));
        }
    }

    /// Append guarded filled rectangles for the standard edit-preview handles.
    ///
    /// The caller owns handle colors while Radiant owns projection, standard
    /// handle order, and finite/visible paint emission.
    pub fn push_standard_handle_fills(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        mut color_for_handle: impl FnMut(TimelineEditHandle) -> Rgba8,
    ) {
        for (handle, rect) in self.standard_handle_rects(mapper, geometry) {
            push_visible_fill_rect(primitives, widget_id, rect, color_for_handle(handle));
        }
    }

    /// Append sampled curve strokes for the standard leading and trailing ramps.
    ///
    /// Radiant owns edit-ramp projection, visibility guards, sample-density
    /// selection, and paint emission. The host owns the domain-specific value
    /// curve and returns a normalized vertical value for each sampled timeline
    /// position. Values outside `0.0..=1.0` are clamped before painting.
    pub fn push_standard_ramp_curve_strokes(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        parts: TimelineEditCurveStrokeParts,
        mut value_at: impl FnMut(TimelineEditRampSide, f32) -> Option<f32>,
    ) -> bool {
        let Some(selection_rect) = self.selection_rect(parts.mapper) else {
            return false;
        };
        let curve_bounds = Rect::from_min_max(
            Point::new(parts.mapper.rect.min.x, selection_rect.min.y),
            Point::new(parts.mapper.rect.max.x, selection_rect.max.y),
        );
        let mut appended = false;
        for (side, start_micros, end_micros) in self.standard_ramp_curve_spans() {
            let pixel_width = (parts.mapper.x_for_micros(end_micros)
                - parts.mapper.x_for_micros(start_micros))
            .abs();
            let steps = curve_stroke_steps(
                pixel_width,
                parts.pixels_per_step,
                parts.min_steps,
                parts.max_steps,
            );
            appended |= push_sampled_curve_stroke(
                primitives,
                SampledCurveStrokeParts::new(
                    parts.widget_id,
                    curve_bounds,
                    steps,
                    parts.color,
                    parts.stroke_width,
                ),
                |t| {
                    let micros = interpolate_micros(start_micros, end_micros, t);
                    if micros < parts.mapper.viewport.start_micros
                        || micros > parts.mapper.viewport.end_micros
                    {
                        return None;
                    }
                    let value = value_at(side, normalized_micros_to_fraction(micros))?;
                    Some(Point::new(
                        parts.mapper.x_for_micros(micros),
                        curve_bounds.max.y - curve_bounds.height() * value.clamp(0.0, 1.0),
                    ))
                },
            );
        }
        appended
    }

    /// Return the first standard edit handle whose rectangle contains `position`.
    pub fn handle_at(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        handles: impl IntoIterator<Item = TimelineEditHandle>,
        position: Point,
    ) -> Option<TimelineEditHandle> {
        handles.into_iter().find(|handle| {
            self.handle_rect(mapper, geometry, *handle)
                .is_some_and(|rect| rect.contains(position))
        })
    }

    /// Return the first standard edit handle whose rectangle contains `position`.
    pub fn standard_handle_at(
        self,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        position: Point,
    ) -> Option<TimelineEditHandle> {
        self.handle_at(
            mapper,
            geometry,
            TimelineEditHandle::standard_order(),
            position,
        )
    }

    fn standard_ramp_curve_spans(self) -> impl Iterator<Item = (TimelineEditRampSide, u32, u32)> {
        let leading = self.leading_end_micros.map(|end| {
            (
                TimelineEditRampSide::Leading,
                self.leading_inner_start_micros
                    .or(self.selection.map(|selection| selection.start_micros))
                    .unwrap_or(end),
                end,
            )
        });
        let trailing = self.trailing_start_micros.map(|start| {
            (
                TimelineEditRampSide::Trailing,
                start,
                self.trailing_inner_end_micros
                    .or(self.selection.map(|selection| selection.end_micros))
                    .unwrap_or(start),
            )
        });
        [leading, trailing]
            .into_iter()
            .flatten()
            .filter(|(_, start, end)| end > start)
    }
}

fn visible_x_for_micros(mapper: TimelineCoordinateMapper, micros: u32) -> Option<f32> {
    if micros < mapper.viewport.start_micros || micros > mapper.viewport.end_micros {
        return None;
    }
    Some(mapper.x_for_micros(micros))
}

fn normalized_handle_size(bounds: Rect, handle_size: f32) -> f32 {
    handle_size
        .max(0.0)
        .min(bounds.width().max(1.0))
        .min(bounds.height().max(1.0))
}

fn curve_stroke_steps(
    pixel_width: f32,
    pixels_per_step: f32,
    min_steps: usize,
    max_steps: usize,
) -> usize {
    let max_steps = max_steps.max(1);
    let min_steps = min_steps.min(max_steps);
    if !pixel_width.is_finite() || pixel_width <= 0.0 {
        return min_steps;
    }
    let pixels_per_step = if pixels_per_step.is_finite() && pixels_per_step > 0.0 {
        pixels_per_step
    } else {
        4.0
    };
    ((pixel_width / pixels_per_step).round() as usize).clamp(min_steps, max_steps)
}

fn interpolate_micros(start: u32, end: u32, t: f32) -> u32 {
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    (start as f32 + (end.saturating_sub(start) as f32 * t)).round() as u32
}

fn normalized_micros_to_fraction(micros: u32) -> f32 {
    micros.min(1_000_000) as f32 / 1_000_000.0
}

fn edit_handle_vertical_band(
    bounds: Rect,
    selection_rect: Rect,
    handle: TimelineEditHandle,
    size: f32,
) -> Rect {
    match handle {
        TimelineEditHandle::LeadingEnd | TimelineEditHandle::TrailingStart => {
            selection_rect.top_edge_strip(size)
        }
        TimelineEditHandle::LeadingStart | TimelineEditHandle::TrailingEnd => {
            selection_rect.bottom_edge_strip(size)
        }
        TimelineEditHandle::LeadingOuterStart | TimelineEditHandle::TrailingOuterEnd => {
            bounds.horizontal_center_strip(size)
        }
    }
}
