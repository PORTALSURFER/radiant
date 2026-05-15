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
    drag_active: bool,
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
        self.drag_active = drag_active;
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
            row = row.with_drop_target(self.drag_active);
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
        drag_active: false,
    }
}
