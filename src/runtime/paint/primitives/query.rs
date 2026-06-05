use super::{
    PaintClipStart, PaintFillPolygon, PaintFillRect, PaintGpuSurface, PaintPrimitive,
    PaintStrokePolyline, PaintStrokeRect, PaintSvg, PaintTextInput, PaintTextRun, SurfacePaintPlan,
};
use crate::{
    gui::types::{Rect, Rgba8},
    widgets::WidgetId,
};
use std::{iter, slice};

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

    /// Iterate over non-clip paint primitives in paint order.
    ///
    /// Use this when tests, automation, or diagnostics need to ask whether a
    /// plan painted visible content without counting clip bookkeeping as paint.
    pub fn paint_primitives(&self) -> impl Iterator<Item = &PaintPrimitive> {
        self.primitives
            .iter()
            .filter(|primitive| primitive.is_paint())
    }

    /// Return whether this plan contains any non-clip paint primitive.
    pub fn contains_paint_primitives(&self) -> bool {
        self.paint_primitives().next().is_some()
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
            .filter(|fill| fill.color.a > 0 && fill.rect.has_finite_positive_area())
    }

    /// Return whether `widget_id` emitted a visible filled rectangle.
    pub fn contains_visible_fill_rect_for_widget(&self, widget_id: WidgetId) -> bool {
        self.primitives
            .iter()
            .any(|primitive| primitive.contains_visible_fill_rect_for_widget(widget_id))
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

    /// Iterate over rectangular regions directly carried by primitives in paint order.
    ///
    /// Batched rectangle primitives contribute every carried rectangle, while
    /// [`PaintPrimitive::rect`] remains the first-rectangle anchor helper for
    /// overlay placement code.
    pub fn rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.primitives.iter().flat_map(PaintPrimitive::rects)
    }

    /// Return whether any rectangle-bearing primitive matches `predicate`.
    pub fn contains_rect_matching(&self, predicate: impl FnMut(Rect) -> bool) -> bool {
        self.rects().any(predicate)
    }

    /// Iterate over rectangular regions carried by non-clip paint primitives.
    pub fn paint_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.paint_primitives().flat_map(PaintPrimitive::rects)
    }

    /// Return whether any non-clip paint rectangle matches `predicate`.
    pub fn contains_paint_rect_matching(&self, predicate: impl FnMut(Rect) -> bool) -> bool {
        self.paint_rects().any(predicate)
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

    /// Return whether this primitive represents paint rather than clip bookkeeping.
    pub const fn is_paint(&self) -> bool {
        !matches!(self, Self::ClipStart(_) | Self::ClipEnd(_))
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

    fn contains_visible_fill_rect_for_widget(&self, widget_id: WidgetId) -> bool {
        match self {
            Self::FillRect(fill) => {
                fill.widget_id == widget_id
                    && fill.color.a > 0
                    && fill.rect.has_finite_positive_area()
            }
            Self::FillRectBatch(fill) => {
                fill.widget_id == widget_id
                    && fill.color.a > 0
                    && fill
                        .rects
                        .iter()
                        .copied()
                        .any(Rect::has_finite_positive_area)
            }
            _ => false,
        }
    }

    /// Iterate over every rectangular region directly carried by this primitive.
    ///
    /// This is allocation-free and differs from [`Self::rect`] only for
    /// batched rectangle primitives, where it yields every rectangle in local
    /// paint order instead of only the first anchor rectangle.
    pub fn rects(&self) -> PrimitiveRects<'_> {
        match self {
            Self::ClipStart(clip) => PrimitiveRects::one(clip.rect),
            Self::ClipEnd(_) => PrimitiveRects::empty(),
            Self::FillRect(fill) => PrimitiveRects::one(fill.rect),
            Self::FillRectBatch(fill) => PrimitiveRects::slice(fill.rects.iter()),
            Self::FillPath(_) => PrimitiveRects::empty(),
            Self::Svg(svg) => PrimitiveRects::one(svg.rect),
            Self::StrokeRect(stroke) => PrimitiveRects::one(stroke.rect),
            Self::StrokeRectBatch(stroke) => PrimitiveRects::slice(stroke.rects.iter()),
            Self::FillPolygon(_) | Self::StrokePolygon(_) | Self::StrokePolyline(_) => {
                PrimitiveRects::empty()
            }
            Self::Text(text) => PrimitiveRects::one(text.rect),
            Self::OverlayPanel(panel) => PrimitiveRects::one(panel.rect),
            Self::TextInput(input) => PrimitiveRects::one(input.rect),
            Self::Image(image) => PrimitiveRects::one(image.rect),
            Self::GpuSurface(surface) => PrimitiveRects::one(surface.rect),
            Self::CustomSurface(surface) => PrimitiveRects::one(surface.rect),
        }
    }
}

/// Allocation-free iterator over the rectangles directly carried by one paint primitive.
pub enum PrimitiveRects<'a> {
    Empty(iter::Empty<Rect>),
    One(iter::Once<Rect>),
    Slice(iter::Copied<slice::Iter<'a, Rect>>),
}

impl<'a> PrimitiveRects<'a> {
    fn empty() -> Self {
        Self::Empty(iter::empty())
    }

    fn one(rect: Rect) -> Self {
        Self::One(iter::once(rect))
    }

    fn slice(rects: slice::Iter<'a, Rect>) -> Self {
        Self::Slice(rects.copied())
    }
}

impl Iterator for PrimitiveRects<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty(rects) => rects.next(),
            Self::One(rects) => rects.next(),
            Self::Slice(rects) => rects.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Empty(rects) => rects.size_hint(),
            Self::One(rects) => rects.size_hint(),
            Self::Slice(rects) => rects.size_hint(),
        }
    }
}

impl ExactSizeIterator for PrimitiveRects<'_> {}
