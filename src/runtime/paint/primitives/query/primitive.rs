use super::super::{
    PaintClipStart, PaintFillPath, PaintFillPolygon, PaintFillRect, PaintGpuSurface,
    PaintPrimitive, PaintStrokePolyline, PaintStrokeRect, PaintSvg, PaintTextInput, PaintTextRun,
};
use crate::{gui::types::Rect, widgets::WidgetId};
use std::{iter, slice};

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

    /// Return the filled path carried by this primitive, if any.
    pub fn fill_path(&self) -> Option<&PaintFillPath> {
        match self {
            Self::FillPath(fill) => Some(fill),
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

    pub(super) fn contains_visible_fill_rect_for_widget(&self, widget_id: WidgetId) -> bool {
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
