use super::{PaintPrimitive, SurfacePaintPlan};
use crate::{gui::types::Rect, widgets::WidgetId};

#[cfg(test)]
#[path = "query/tests.rs"]
mod tests;

impl SurfacePaintPlan {
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
}

impl PaintPrimitive {
    /// Return the widget that emitted this primitive when the primitive belongs
    /// to a widget rather than a container clip.
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::ClipStart(_) | Self::ClipEnd(_) => None,
            Self::FillRect(fill) => Some(fill.widget_id),
            Self::FillPath(fill) => Some(fill.widget_id),
            Self::Svg(svg) => Some(svg.widget_id),
            Self::StrokeRect(stroke) => Some(stroke.widget_id),
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
            Self::FillPath(_) => None,
            Self::Svg(svg) => Some(svg.rect),
            Self::StrokeRect(stroke) => Some(stroke.rect),
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
