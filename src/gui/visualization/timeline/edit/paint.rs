use super::{
    TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry, TimelineEditPreview,
    TimelineEditRampSide, TimelineEditRegion, TimelineEditRegionGeometry,
};
use crate::{
    gui::{
        types::{Point, Rect, Rgba8},
        visualization::{SampledCurveStrokeParts, push_sampled_curve_stroke},
    },
    runtime::{PaintPrimitive, push_visible_fill_rect},
    widgets::WidgetId,
};

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

/// Paint colors for standard timeline edit-preview affordances.
///
/// Hosts supply the base color that matches their domain or theme. Radiant owns
/// the standard inner/outer region split and the derived handle and curve
/// colors so app widgets do not need to duplicate edit-preview taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineEditPaintStyle {
    /// Base color used for derived edit-preview affordances.
    pub base_color: Rgba8,
    /// Alpha for regions inside the selected interval.
    pub inner_region_alpha: u8,
    /// Alpha for regions outside the selected interval.
    pub outer_region_alpha: u8,
    /// Alpha for standard edit-preview handles.
    pub handle_alpha: u8,
    /// Alpha for standard edit-preview ramp curves.
    pub curve_alpha: u8,
}

impl TimelineEditPaintStyle {
    /// Build a standard edit-preview style from a host-supplied base color.
    pub const fn new(base_color: Rgba8) -> Self {
        Self {
            base_color,
            inner_region_alpha: 52,
            outer_region_alpha: 38,
            handle_alpha: 205,
            curve_alpha: 225,
        }
    }

    /// Override the inner and outer region alphas.
    pub const fn region_alphas(mut self, inner_region_alpha: u8, outer_region_alpha: u8) -> Self {
        self.inner_region_alpha = inner_region_alpha;
        self.outer_region_alpha = outer_region_alpha;
        self
    }

    /// Override the standard edit-preview handle alpha.
    pub const fn handle_alpha(mut self, handle_alpha: u8) -> Self {
        self.handle_alpha = handle_alpha;
        self
    }

    /// Override the standard edit-preview curve alpha.
    pub const fn curve_alpha(mut self, curve_alpha: u8) -> Self {
        self.curve_alpha = curve_alpha;
        self
    }

    /// Return the color for a standard edit-preview region.
    pub const fn region_color(self, region: TimelineEditRegion) -> Rgba8 {
        match region {
            TimelineEditRegion::LeadingInner | TimelineEditRegion::TrailingInner => {
                self.base_color.with_alpha(self.inner_region_alpha)
            }
            TimelineEditRegion::LeadingOuter | TimelineEditRegion::TrailingOuter => {
                self.base_color.with_alpha(self.outer_region_alpha)
            }
        }
    }

    /// Return the color for a standard edit-preview handle.
    pub const fn handle_color(self, _handle: TimelineEditHandle) -> Rgba8 {
        self.base_color.with_alpha(self.handle_alpha)
    }

    /// Return the color for a standard edit-preview ramp curve.
    pub const fn curve_color(self) -> Rgba8 {
        self.base_color.with_alpha(self.curve_alpha)
    }

    /// Build curve stroke parts with this style's standard curve color.
    pub const fn curve_stroke_parts(
        self,
        widget_id: WidgetId,
        mapper: TimelineCoordinateMapper,
        stroke_width: f32,
    ) -> TimelineEditCurveStrokeParts {
        TimelineEditCurveStrokeParts::new(widget_id, mapper, self.curve_color(), stroke_width)
    }
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

impl TimelineEditPreview {
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

    /// Append guarded filled rectangles for the standard edit-preview regions
    /// using a reusable Radiant paint style.
    pub fn push_standard_styled_region_fills(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditRegionGeometry,
        style: TimelineEditPaintStyle,
    ) {
        self.push_standard_region_fills(primitives, widget_id, mapper, geometry, |region| {
            style.region_color(region)
        });
    }

    /// Append guarded filled rectangles for the standard edit-preview handles
    /// using a reusable Radiant paint style.
    pub fn push_standard_styled_handle_fills(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        mapper: TimelineCoordinateMapper,
        geometry: TimelineEditHandleGeometry,
        style: TimelineEditPaintStyle,
    ) {
        self.push_standard_handle_fills(primitives, widget_id, mapper, geometry, |handle| {
            style.handle_color(handle)
        });
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
