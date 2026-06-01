use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{
        FocusBehavior, InteractiveRowMessage, InteractiveRowWidget, PaintBounds, WidgetProminence,
        WidgetSizing, WidgetStyle,
    },
};

/// Builder for selectable, draggable, droppable dense rows.
pub struct InteractiveRowBuilder {
    style: Option<WidgetStyle>,
    sizing: WidgetSizing,
    focus: Option<FocusBehavior>,
    paint_bounds: Option<PaintBounds>,
    paints_focus: Option<bool>,
    paints_state_layers: Option<bool>,
    draggable: bool,
    droppable: bool,
    drop_hover: bool,
    drag_active: bool,
    drag_source: bool,
    drag_source_motion: bool,
    suppress_hover: bool,
    clear_hover_on_sync: bool,
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

    /// Override the row widget sizing.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// Override keyboard focus behavior.
    pub fn focus(mut self, focus: FocusBehavior) -> Self {
        self.focus = Some(focus);
        self
    }

    /// Override how this row's paint is bounded.
    pub fn paint_bounds(mut self, bounds: PaintBounds) -> Self {
        self.paint_bounds = Some(bounds);
        self
    }

    /// Control whether this row paints focus affordances.
    pub fn paint_focus(mut self, paint: bool) -> Self {
        self.paints_focus = Some(paint);
        self
    }

    /// Control whether this row paints its built-in hover and pressed layers.
    pub fn paint_state_layers(mut self, paint: bool) -> Self {
        self.paints_state_layers = Some(paint);
        self
    }

    /// Configure this row as an input-only layer for app-owned custom painting.
    ///
    /// The row still routes pointer, keyboard, drag, and drop interactions, but
    /// it does not request keyboard focus or paint Radiant's built-in focus and
    /// hover/pressed layers. Custom composite widgets can use this preset when
    /// they want generic row input behavior with their own visual state model.
    pub fn custom_paint_hit_target(mut self) -> Self {
        self.focus = Some(FocusBehavior::None);
        self.paint_bounds = Some(PaintBounds::ClipToRect);
        self.paints_focus = Some(false);
        self.paints_state_layers = Some(false);
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

    /// Configure whether this row is a drop target and whether hover-drop
    /// messages should be emitted.
    pub fn drop_target_mode(mut self, drag_active: bool, hover_messages: bool) -> Self {
        self.droppable = drag_active;
        self.drop_hover = drag_active && hover_messages;
        self.drag_active = drag_active;
        self
    }

    /// Mark whether a related row drag is active in this row's container.
    pub fn drag_active(mut self, active: bool) -> Self {
        self.drag_active = active;
        self
    }

    /// Mark this row as the source of the current container drag.
    pub fn drag_source(mut self, source: bool) -> Self {
        self.drag_source = source;
        self
    }

    /// Emit drag move messages while this row remains the active drag source.
    pub fn drag_source_motion(mut self, enabled: bool) -> Self {
        self.drag_source_motion = enabled;
        self
    }

    /// Ignore hover updates for this row while preserving activation and drag behavior.
    pub fn suppress_hover(mut self, suppress: bool) -> Self {
        self.suppress_hover = suppress;
        self
    }

    /// Clear retained hover state when this row is synchronized from a previous tree.
    pub fn clear_hover_on_sync(mut self) -> Self {
        self.clear_hover_on_sync = true;
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
        let style = self.style;
        let row = self.widget();
        let mut node = view_node_from_widget(MappedWidget::new(
            row,
            WidgetMessageMapper::interactive_row(map),
        ));
        node.style = style;
        node
    }

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

/// Build an interactive dense row hit surface.
pub fn interactive_row() -> InteractiveRowBuilder {
    InteractiveRowBuilder {
        style: None,
        sizing: WidgetSizing::fixed(crate::layout::Vector2::new(1.0, 22.0)),
        focus: None,
        paint_bounds: None,
        paints_focus: None,
        paints_state_layers: None,
        draggable: false,
        droppable: false,
        drop_hover: false,
        drag_active: false,
        drag_source: false,
        drag_source_motion: false,
        suppress_hover: false,
        clear_hover_on_sync: false,
        activation_modifiers: false,
        pointer_motion_during_interaction: false,
        pointer_motion_active: false,
    }
}
