use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{
        InteractiveRowMessage, InteractiveRowWidget, WidgetProminence, WidgetSizing, WidgetStyle,
    },
};

/// Builder for selectable, draggable, droppable dense rows.
pub struct InteractiveRowBuilder {
    style: Option<WidgetStyle>,
    draggable: bool,
    droppable: bool,
    drop_hover: bool,
    drag_active: bool,
    activation_modifiers: bool,
    pointer_motion_during_interaction: bool,
    pointer_motion_active: bool,
}

impl InteractiveRowBuilder {
    /// Apply an explicit widget style before binding this row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Emit drag lifecycle messages from this row.
    pub fn draggable(mut self) -> Self {
        self.draggable = true;
        self
    }

    /// Emit drop and hover-drop-target messages.
    pub fn droppable(mut self, drag_active: bool) -> Self {
        self.droppable = true;
        self.drop_hover = true;
        self.drag_active = drag_active;
        self
    }

    /// Emit drop messages without hover-drop-target messages.
    pub fn drop_only(mut self, drag_active: bool) -> Self {
        self.droppable = true;
        self.drop_hover = false;
        self.drag_active = drag_active;
        self
    }

    /// Include primary-release modifier state in pointer activation messages.
    pub fn activation_modifiers(mut self) -> Self {
        self.activation_modifiers = true;
        self
    }

    /// Restrict pointer-motion routing to active row interactions.
    pub fn pointer_motion_during_interaction(mut self) -> Self {
        self.pointer_motion_during_interaction = true;
        self
    }

    /// Mark app-owned interaction state that should keep pointer motion routed.
    pub fn pointer_motion_active(mut self, active: bool) -> Self {
        self.pointer_motion_active = active;
        self
    }

    /// Emit mapped host messages for row interactions.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut row = InteractiveRowWidget::new(
            0,
            WidgetSizing::fixed(crate::layout::Vector2::new(1.0, 22.0)),
        );
        if self.draggable {
            row = row.with_drag();
        }
        if self.droppable {
            row = if self.drop_hover {
                row.with_drop_target(self.drag_active)
            } else {
                row.with_drop_only(self.drag_active)
            };
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
        let mut node = view_node_from_widget(MappedWidget::new(
            row,
            WidgetMessageMapper::interactive_row(map),
        ));
        node.style = self.style;
        node
    }
}

/// Build an interactive dense row hit surface.
pub fn interactive_row() -> InteractiveRowBuilder {
    InteractiveRowBuilder {
        style: None,
        draggable: false,
        droppable: false,
        drop_hover: false,
        drag_active: false,
        activation_modifiers: false,
        pointer_motion_during_interaction: false,
        pointer_motion_active: false,
    }
}
