use super::super::SurfacePaintPlan;
use crate::{gui::types::Rect, widgets::WidgetId};

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
