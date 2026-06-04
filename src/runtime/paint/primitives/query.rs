use super::{
    PaintClipStart, PaintFillPolygon, PaintFillRect, PaintGpuSurface, PaintPrimitive,
    PaintStrokePolyline, PaintStrokeRect, PaintSvg, PaintTextInput, PaintTextRun, SurfacePaintPlan,
};
use crate::{
    gui::types::{Rect, Rgba8},
    widgets::WidgetId,
};

#[cfg(test)]
#[path = "query/tests.rs"]
mod tests;

impl SurfacePaintPlan {
    /// Iterate over text runs emitted by this paint plan in paint order.
    pub fn text_runs(&self) -> impl Iterator<Item = &PaintTextRun> {
        self.primitives.iter().filter_map(PaintPrimitive::text_run)
    }

    /// Iterate over visible text labels emitted by this paint plan in paint order.
    pub fn text_labels(&self) -> impl Iterator<Item = &str> {
        self.text_runs().map(|run| run.text.as_str())
    }

    /// Collect visible text labels emitted by this paint plan in paint order.
    ///
    /// Use this in tests, automation snapshots, or diagnostics that need owned
    /// labels for failure output without repeating text-run mapping boilerplate.
    pub fn text_label_strings(&self) -> Vec<String> {
        self.text_labels().map(str::to_string).collect()
    }

    /// Return the first text run with exactly matching visible text.
    pub fn first_text_run(&self, text: &str) -> Option<&PaintTextRun> {
        self.text_runs().find(|run| run.text.as_str() == text)
    }

    /// Return the first text run with exactly matching visible text whose
    /// rectangle begins at or after `min_x`.
    pub fn first_text_run_after_x(&self, text: &str, min_x: f32) -> Option<&PaintTextRun> {
        self.text_runs()
            .find(|run| run.text.as_str() == text && run.rect.min.x >= min_x)
    }

    /// Return whether this paint plan contains a text run with exactly matching
    /// visible text.
    pub fn contains_text(&self, text: &str) -> bool {
        self.first_text_run(text).is_some()
    }

    /// Return whether this paint plan contains exactly matching visible text
    /// whose rectangle begins at or after `min_x`.
    pub fn contains_text_after_x(&self, text: &str, min_x: f32) -> bool {
        self.first_text_run_after_x(text, min_x).is_some()
    }

    /// Return the rectangle for the first text run with exactly matching
    /// visible text.
    pub fn first_text_rect(&self, text: &str) -> Option<Rect> {
        self.first_text_run(text).map(|run| run.rect)
    }

    /// Return the color for the first text run with exactly matching visible
    /// text.
    pub fn first_text_color(&self, text: &str) -> Option<Rgba8> {
        self.first_text_run(text).map(|run| run.color)
    }

    /// Iterate over native text-input paint primitives in paint order.
    pub fn text_inputs(&self) -> impl Iterator<Item = &PaintTextInput> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::text_input)
    }

    /// Return the first native text-input paint primitive in paint order.
    pub fn first_text_input(&self) -> Option<&PaintTextInput> {
        self.text_inputs().next()
    }

    /// Return whether this paint plan contains any native text-input paint
    /// primitive.
    pub fn contains_text_input(&self) -> bool {
        self.first_text_input().is_some()
    }

    /// Iterate over rectangular clip-start primitives in paint order.
    pub fn clip_starts(&self) -> impl Iterator<Item = &PaintClipStart> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::clip_start)
    }

    /// Iterate over single filled-rectangle primitives in paint order.
    ///
    /// Batched rectangle primitives remain available through `primitives` when
    /// callers need to inspect every rectangle inside a batch.
    pub fn fill_rects(&self) -> impl Iterator<Item = &PaintFillRect> {
        self.primitives.iter().filter_map(PaintPrimitive::fill_rect)
    }

    /// Iterate over single filled-rectangle primitives emitted by `widget_id`
    /// in paint order.
    pub fn fill_rects_for_widget(
        &self,
        widget_id: WidgetId,
    ) -> impl Iterator<Item = &PaintFillRect> {
        self.fill_rects()
            .filter(move |fill| fill.widget_id == widget_id)
    }

    /// Iterate over visible single filled-rectangle primitives emitted by
    /// `widget_id` in paint order.
    ///
    /// A visible fill has non-zero alpha and a finite positive rectangle.
    pub fn visible_fill_rects_for_widget(
        &self,
        widget_id: WidgetId,
    ) -> impl Iterator<Item = &PaintFillRect> {
        self.fill_rects_for_widget(widget_id)
            .filter(|fill| fill.color.a > 0 && rect_has_positive_area(fill.rect))
    }

    /// Return whether `widget_id` emitted a visible single filled rectangle.
    pub fn contains_visible_fill_rect_for_widget(&self, widget_id: WidgetId) -> bool {
        self.visible_fill_rects_for_widget(widget_id)
            .next()
            .is_some()
    }

    /// Iterate over single stroked-rectangle primitives in paint order.
    ///
    /// Batched rectangle primitives remain available through `primitives` when
    /// callers need to inspect every rectangle inside a batch.
    pub fn stroke_rects(&self) -> impl Iterator<Item = &PaintStrokeRect> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::stroke_rect)
    }

    /// Iterate over single stroked-rectangle primitives emitted by `widget_id`
    /// in paint order.
    pub fn stroke_rects_for_widget(
        &self,
        widget_id: WidgetId,
    ) -> impl Iterator<Item = &PaintStrokeRect> {
        self.stroke_rects()
            .filter(move |stroke| stroke.widget_id == widget_id)
    }

    /// Iterate over filled-polygon primitives in paint order.
    pub fn fill_polygons(&self) -> impl Iterator<Item = &PaintFillPolygon> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::fill_polygon)
    }

    /// Iterate over filled-polygon primitives emitted by `widget_id` in paint
    /// order.
    pub fn fill_polygons_for_widget(
        &self,
        widget_id: WidgetId,
    ) -> impl Iterator<Item = &PaintFillPolygon> {
        self.fill_polygons()
            .filter(move |fill| fill.widget_id == widget_id)
    }

    /// Iterate over visible filled-polygon primitives emitted by `widget_id` in
    /// paint order.
    pub fn visible_fill_polygons_for_widget(
        &self,
        widget_id: WidgetId,
    ) -> impl Iterator<Item = &PaintFillPolygon> {
        self.fill_polygons_for_widget(widget_id)
            .filter(|fill| fill.color.a > 0)
    }

    /// Return whether `widget_id` emitted a visible filled polygon.
    pub fn contains_visible_fill_polygon_for_widget(&self, widget_id: WidgetId) -> bool {
        self.visible_fill_polygons_for_widget(widget_id)
            .next()
            .is_some()
    }

    /// Iterate over stroked-polyline primitives in paint order.
    pub fn stroke_polylines(&self) -> impl Iterator<Item = &PaintStrokePolyline> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::stroke_polyline)
    }

    /// Iterate over retained SVG primitives in paint order.
    pub fn svgs(&self) -> impl Iterator<Item = &PaintSvg> {
        self.primitives.iter().filter_map(PaintPrimitive::svg)
    }

    /// Iterate over retained SVG primitives emitted by `widget_id` in paint
    /// order.
    pub fn svgs_for_widget(&self, widget_id: WidgetId) -> impl Iterator<Item = &PaintSvg> {
        self.svgs().filter(move |svg| svg.widget_id == widget_id)
    }

    /// Return the rectangle for the first retained SVG emitted by `widget_id`.
    pub fn first_svg_rect_for_widget(&self, widget_id: WidgetId) -> Option<Rect> {
        self.svgs_for_widget(widget_id).map(|svg| svg.rect).next()
    }

    /// Iterate over retained GPU surface primitives in paint order.
    pub fn gpu_surfaces(&self) -> impl Iterator<Item = &PaintGpuSurface> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::gpu_surface)
    }

    /// Return the first rectangular paint region emitted by `widget_id`.
    ///
    /// Transient overlays can use this to anchor lightweight frame-time paint
    /// to the cached surface plan without matching individual primitive
    /// variants in their per-frame animation path. This returns the first
    /// rectangle-like primitive for the widget in paint order, which matches
    /// retained GPU surfaces, custom surfaces, images, text, input fields,
    /// overlay panels, and rectangular fills/strokes.
    pub fn first_widget_rect(&self, widget_id: WidgetId) -> Option<Rect> {
        self.primitives.iter().find_map(|primitive| {
            (primitive.widget_id() == Some(widget_id))
                .then(|| primitive.rect())
                .flatten()
        })
    }

    /// Return the first rectangular paint region for the first widget ID in
    /// caller-provided priority order that has a rectangular primitive.
    ///
    /// Transient overlays can use this when a visual should prefer a primary
    /// surface but fall back to another equivalent anchor without repeating
    /// `or_else(...)` chains in the per-frame path.
    pub fn first_widget_rect_by_priority(
        &self,
        widget_ids: impl IntoIterator<Item = WidgetId>,
    ) -> Option<Rect> {
        widget_ids
            .into_iter()
            .find_map(|widget_id| self.first_widget_rect(widget_id))
    }
}

fn rect_has_positive_area(rect: Rect) -> bool {
    rect.width().is_finite()
        && rect.height().is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

impl PaintPrimitive {
    /// Return the text run carried by this primitive, if it is text.
    pub fn text_run(&self) -> Option<&PaintTextRun> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Return the text-input paint payload carried by this primitive, if any.
    pub fn text_input(&self) -> Option<&PaintTextInput> {
        match self {
            Self::TextInput(input) => Some(input),
            _ => None,
        }
    }

    /// Return the clip-start payload carried by this primitive, if any.
    pub fn clip_start(&self) -> Option<&PaintClipStart> {
        match self {
            Self::ClipStart(clip) => Some(clip),
            _ => None,
        }
    }

    /// Return the filled rectangle carried by this primitive, if any.
    pub fn fill_rect(&self) -> Option<&PaintFillRect> {
        match self {
            Self::FillRect(fill) => Some(fill),
            _ => None,
        }
    }

    /// Return the stroked rectangle carried by this primitive, if any.
    pub fn stroke_rect(&self) -> Option<&PaintStrokeRect> {
        match self {
            Self::StrokeRect(stroke) => Some(stroke),
            _ => None,
        }
    }

    /// Return the filled polygon carried by this primitive, if any.
    pub fn fill_polygon(&self) -> Option<&PaintFillPolygon> {
        match self {
            Self::FillPolygon(fill) => Some(fill),
            _ => None,
        }
    }

    /// Return the stroked polyline carried by this primitive, if any.
    pub fn stroke_polyline(&self) -> Option<&PaintStrokePolyline> {
        match self {
            Self::StrokePolyline(stroke) => Some(stroke),
            _ => None,
        }
    }

    /// Return the retained SVG payload carried by this primitive, if any.
    pub fn svg(&self) -> Option<&PaintSvg> {
        match self {
            Self::Svg(svg) => Some(svg),
            _ => None,
        }
    }

    /// Return the retained GPU surface payload carried by this primitive, if
    /// any.
    pub fn gpu_surface(&self) -> Option<&PaintGpuSurface> {
        match self {
            Self::GpuSurface(surface) => Some(surface),
            _ => None,
        }
    }

    /// Return the widget that emitted this primitive when the primitive belongs
    /// to a widget rather than a container clip.
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::ClipStart(_) | Self::ClipEnd(_) => None,
            Self::FillRect(fill) => Some(fill.widget_id),
            Self::FillRectBatch(fill) => Some(fill.widget_id),
            Self::FillPath(fill) => Some(fill.widget_id),
            Self::Svg(svg) => Some(svg.widget_id),
            Self::StrokeRect(stroke) => Some(stroke.widget_id),
            Self::StrokeRectBatch(stroke) => Some(stroke.widget_id),
            Self::FillPolygon(fill) => Some(fill.widget_id),
            Self::StrokePolygon(stroke) => Some(stroke.widget_id),
            Self::StrokePolyline(stroke) => Some(stroke.widget_id),
            Self::Text(text) => Some(text.widget_id),
            Self::OverlayPanel(panel) => Some(panel.widget_id),
            Self::TextInput(input) => Some(input.widget_id),
            Self::Image(image) => Some(image.widget_id),
            Self::GpuSurface(surface) => Some(surface.widget_id),
            Self::CustomSurface(surface) => Some(surface.widget_id),
        }
    }

    /// Return the rectangular region directly carried by this primitive.
    ///
    /// Vector paths and point-list primitives can still be inspected through
    /// their own fields. This helper intentionally stays allocation-free and
    /// covers the rectangle-bearing primitives used as overlay anchors.
    pub fn rect(&self) -> Option<Rect> {
        match self {
            Self::ClipStart(clip) => Some(clip.rect),
            Self::ClipEnd(_) => None,
            Self::FillRect(fill) => Some(fill.rect),
            Self::FillRectBatch(fill) => fill.rects.first().copied(),
            Self::FillPath(_) => None,
            Self::Svg(svg) => Some(svg.rect),
            Self::StrokeRect(stroke) => Some(stroke.rect),
            Self::StrokeRectBatch(stroke) => stroke.rects.first().copied(),
            Self::FillPolygon(_) | Self::StrokePolygon(_) | Self::StrokePolyline(_) => None,
            Self::Text(text) => Some(text.rect),
            Self::OverlayPanel(panel) => Some(panel.rect),
            Self::TextInput(input) => Some(input.rect),
            Self::Image(image) => Some(image.rect),
            Self::GpuSurface(surface) => Some(surface.rect),
            Self::CustomSurface(surface) => Some(surface.rect),
        }
    }
}
