use crate::{
    layout::Vector2,
    widgets::{InteractiveRowActions, WidgetSizing},
};

use super::InteractiveRowBuilder;

/// Build an interactive dense row hit surface.
pub fn interactive_row() -> InteractiveRowBuilder {
    InteractiveRowBuilder {
        style: None,
        sizing: WidgetSizing::fixed(Vector2::new(1.0, 22.0)),
        focus: None,
        paint_bounds: None,
        paints_focus: None,
        paints_state_layers: None,
        draggable: false,
        droppable: false,
        drop_hover: false,
        clear_drop_on_hover: false,
        drag_active: false,
        drag_source: false,
        drag_source_motion: false,
        suppress_hover: false,
        hover_messages: false,
        clear_hover_on_sync: false,
        activation_modifiers: false,
        pointer_motion_during_interaction: false,
        pointer_motion_active: false,
    }
}

/// Build an empty interactive row action router.
pub fn row_actions<Message>() -> InteractiveRowActions<Message> {
    InteractiveRowActions::new()
}
