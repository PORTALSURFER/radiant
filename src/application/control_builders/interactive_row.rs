use crate::{
    application::{MappedWidget, ViewNode, input_underlay, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{
        FocusBehavior, InteractiveRowMessage, InteractiveRowWidget, PaintBounds, WidgetId,
        WidgetProminence, WidgetSizing, WidgetStyle,
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

    /// Configure a host-tracked drop target.
    ///
    /// While `active_target` is true, the row still accepts the eventual drop
    /// but suppresses duplicate hover-drop messages and keeps pointer-motion
    /// routing active through the host-owned interaction state.
    pub fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.pointer_motion_during_interaction = true;
        self.pointer_motion_active = active_target;
        self.droppable = drag_active;
        self.drop_hover = drag_active && !active_target;
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
        self.with_message_mapper(WidgetMessageMapper::interactive_row(map))
    }

    /// Emit host messages for selected row interactions.
    pub fn filter_mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.with_message_mapper(WidgetMessageMapper::interactive_row_filtered(map))
    }

    fn with_message_mapper<Message: 'static>(
        self,
        messages: WidgetMessageMapper<Message>,
    ) -> ViewNode<Message> {
        let style = self.style;
        let row = self.widget();
        let mut node = view_node_from_widget(MappedWidget::new(row, messages));
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

/// Builder for arbitrary row content backed by a generic interactive row.
pub struct InteractiveRowUnderlayBuilder<Message> {
    content: ViewNode<Message>,
    row: InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    style: Option<WidgetStyle>,
}

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
    /// Configure the backing interactive row before binding messages.
    pub fn row(
        mut self,
        configure: impl FnOnce(InteractiveRowBuilder) -> InteractiveRowBuilder,
    ) -> Self {
        self.row = configure(self.row);
        self
    }

    /// Assign a stable widget id to the backing interactive row.
    pub fn input_id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self
    }

    /// Apply an explicit style to the backing interactive row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Emit mapped host messages for row interactions.
    pub fn mapped(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            style,
        } = self;
        Self::finish_parts(content, row.mapped(map), input_id, style)
    }

    /// Emit host messages for selected row interactions.
    pub fn filter_mapped(
        self,
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            style,
        } = self;
        Self::finish_parts(content, row.filter_mapped(map), input_id, style)
    }

    fn finish_parts(
        content: ViewNode<Message>,
        mut input: ViewNode<Message>,
        input_id: Option<WidgetId>,
        style: Option<WidgetStyle>,
    ) -> ViewNode<Message> {
        if let Some(id) = input_id {
            input = input.id(id);
        }
        if let Some(style) = style {
            input = input.style(style);
        }
        input_underlay(content, input)
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

/// Build arbitrary visible content backed by an interactive row underlay.
///
/// The content remains visible above the row, while the backing row owns
/// activation, secondary activation, drag, drop, focus, and row feedback paint.
pub fn interactive_row_underlay<Message: 'static>(
    content: ViewNode<Message>,
) -> InteractiveRowUnderlayBuilder<Message> {
    InteractiveRowUnderlayBuilder {
        content,
        row: interactive_row(),
        input_id: None,
        style: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, text},
        gui::types::{Point, Rect},
        layout::Vector2,
        runtime::{PaintPrimitive, UiSurface},
        widgets::WidgetOutput,
    };

    #[derive(Clone, Debug, PartialEq)]
    enum DemoMessage {
        Activate,
    }

    #[test]
    fn interactive_row_underlay_preserves_input_widget_identity() {
        let view = interactive_row_underlay(text("Collection"))
            .input_id(770)
            .filter_mapped(|message| {
                message
                    .is_single_activation()
                    .then_some(DemoMessage::Activate)
            })
            .size(140.0, 22.0);

        assert_eq!(
            view.view_dispatch_widget_output(
                770,
                WidgetOutput::typed(InteractiveRowMessage::Activate),
            ),
            Some(DemoMessage::Activate)
        );
    }

    #[test]
    fn tracked_drop_target_accepts_drop_without_repeating_hover_for_active_target() {
        let view = interactive_row().tracked_drop_target(true, true).widget();

        assert!(view.props.droppable);
        assert!(view.props.drag_active);
        assert!(!view.props.drop_hover);
        assert!(view.props.pointer_motion_active);
        assert_eq!(
            view.props.pointer_motion,
            crate::widgets::InteractiveRowPointerMotion::DuringInteraction
        );
    }

    #[test]
    fn tracked_drop_target_emits_hover_for_candidate_target() {
        let view = interactive_row().tracked_drop_target(true, false).widget();

        assert!(view.props.droppable);
        assert!(view.props.drag_active);
        assert!(view.props.drop_hover);
        assert!(!view.props.pointer_motion_active);
        assert_eq!(
            view.props.pointer_motion,
            crate::widgets::InteractiveRowPointerMotion::DuringInteraction
        );
    }

    #[test]
    fn interactive_row_underlay_paints_visible_content() {
        let frame = UiSurface::new(
            interactive_row_underlay(text("Collection"))
                .mapped(|_| ())
                .size(140.0, 22.0)
                .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
            &Default::default(),
        );

        let paints_label = frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Collection"),
        );

        assert!(
            paints_label,
            "interactive row underlay should paint visible content"
        );
    }
}
