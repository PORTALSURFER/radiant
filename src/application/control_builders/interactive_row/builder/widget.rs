use crate::widgets::InteractiveRowWidget;

use super::InteractiveRowBuilder;

impl InteractiveRowBuilder {
    /// Build the configured row widget for custom composite widgets.
    ///
    /// This is useful when an application needs generic Radiant row input
    /// behavior but owns specialized painting or layout around the hit target.
    pub fn widget(self) -> InteractiveRowWidget {
        let mut row = InteractiveRowWidget::new(0, self.sizing);
        if self.draggable {
            row = row.with_drag();
        }
        if self.drag_active {
            row = row.with_drag_active(true);
        }
        if self.drag_source {
            row = row.with_drag_source(true);
        }
        if self.drag_source_motion {
            row = row.with_drag_source_motion(true);
        }
        if self.suppress_hover {
            row = row.suppress_hover(true);
        }
        if self.clear_hover_on_sync {
            row = row.clear_hover_on_sync();
        }
        if self.droppable {
            row = row.with_drop_target_mode(self.drag_active, self.drop_hover);
        }
        if self.activation_modifiers {
            row = row.with_activation_modifiers();
        }
        if self.pointer_motion_during_interaction {
            row = row.with_pointer_motion_during_interaction();
        }
        if self.pointer_motion_active {
            row = row.with_pointer_motion_active(true);
        }
        if let Some(focus) = self.focus {
            row.common.focus = focus;
        }
        if let Some(bounds) = self.paint_bounds {
            row.common.paint.bounds = bounds;
        }
        if let Some(paint) = self.paints_focus {
            row.common.paint.paints_focus = paint;
        }
        if let Some(paint) = self.paints_state_layers {
            row.common.paint.paints_state_layers = paint;
        }
        row
    }
}
