use super::super::{
    PaintClipStart, PaintFillPolygon, PaintFillRect, PaintPrimitive, PaintStrokePolyline,
    PaintStrokeRect, PaintSvg, SurfacePaintPlan,
};
use crate::{gui::types::Rect, widgets::WidgetId};

impl SurfacePaintPlan {
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
}
