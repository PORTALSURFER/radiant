use crate::{
    application::{
        MappedWidget, StateAction, ViewNode, danger_style, default_badge_sizing, input_overlay,
        interactive_row, primary_style, view_node_from_widget,
    },
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{
        BadgeMessage, BadgeWidget, FocusBehavior, InteractiveRowMessage, PaintBounds,
        WidgetProminence, WidgetSizing, WidgetStyle,
    },
};

/// Builder for badges that can emit messages or mutate state directly.
pub struct BadgeBuilder {
    label: PaintText,
    style: Option<WidgetStyle>,
    active: bool,
}

impl BadgeBuilder {
    /// Apply an explicit widget style before binding this badge.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone.
    pub fn danger(self) -> Self {
        self.style(danger_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Mark this badge as active for visual state resolution.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Use this badge as the visual layer for an interactive badge or pill.
    ///
    /// Interactive badges preserve badge styling while routing Radiant's generic
    /// dense-row interactions such as activation, secondary activation, drag,
    /// drop, and drop-hover through a transparent input layer.
    pub fn interactive(self) -> InteractiveBadgeBuilder {
        InteractiveBadgeBuilder {
            badge: self,
            row: interactive_row(),
        }
    }

    /// Build a passive badge view without host messages.
    pub fn passive<Message: 'static>(self) -> ViewNode<Message> {
        self.passive_view()
    }

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.mapped(move |_| message.clone())
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let sizing = default_badge_sizing(&self.label);
        let badge = BadgeWidget::new(0, self.label, sizing).with_active(self.active);
        let mut node =
            view_node_from_widget(MappedWidget::new(badge, WidgetMessageMapper::badge(map)));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when activated.
    pub fn on_click<State: 'static>(
        self,
        apply: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        self.message(StateAction::new(apply))
    }

    fn passive_view<Message: 'static>(self) -> ViewNode<Message> {
        let sizing = default_badge_sizing(&self.label);
        let badge = BadgeWidget::new(0, self.label, sizing).with_active(self.active);
        let mut node = view_node_from_widget(badge);
        node.style = self.style;
        node
    }
}

/// Builder for badge visuals with rich row-style interactions.
pub struct InteractiveBadgeBuilder {
    badge: BadgeBuilder,
    row: super::interactive_row::InteractiveRowBuilder,
}

impl InteractiveBadgeBuilder {
    /// Apply an explicit widget style to the badge visual.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.badge = self.badge.style(style);
        self
    }

    /// Use the accent tone and strong badge prominence.
    pub fn primary(mut self) -> Self {
        self.badge = self.badge.primary();
        self
    }

    /// Use the danger tone for the badge visual.
    pub fn danger(mut self) -> Self {
        self.badge = self.badge.danger();
        self
    }

    /// Use a lower-prominence badge treatment.
    pub fn subtle(mut self) -> Self {
        self.badge = self.badge.subtle();
        self
    }

    /// Mark the badge visual as active.
    pub fn active(mut self, active: bool) -> Self {
        self.badge = self.badge.active(active);
        self
    }

    /// Override the transparent interaction surface sizing.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.row = self.row.sizing(sizing);
        self
    }

    /// Override keyboard focus behavior for the interaction surface.
    pub fn focus(mut self, focus: FocusBehavior) -> Self {
        self.row = self.row.focus(focus);
        self
    }

    /// Override how the interaction surface paint is bounded.
    pub fn paint_bounds(mut self, bounds: PaintBounds) -> Self {
        self.row = self.row.paint_bounds(bounds);
        self
    }

    /// Control whether the interaction surface paints focus affordances.
    pub fn paint_focus(mut self, paint: bool) -> Self {
        self.row = self.row.paint_focus(paint);
        self
    }

    /// Control whether the interaction surface paints built-in state layers.
    pub fn paint_state_layers(mut self, paint: bool) -> Self {
        self.row = self.row.paint_state_layers(paint);
        self
    }

    /// Configure this badge as an input-only layer for app-owned badge painting.
    pub fn custom_paint_hit_target(mut self) -> Self {
        self.row = self.row.custom_paint_hit_target();
        self
    }

    /// Emit drag lifecycle messages from this badge.
    pub fn draggable(mut self) -> Self {
        self.row = self.row.draggable();
        self
    }

    /// Emit drop and hover-drop-target messages.
    pub fn droppable(mut self, drag_active: bool) -> Self {
        self.row = self.row.droppable(drag_active);
        self
    }

    /// Emit drop messages without hover-drop-target messages.
    pub fn drop_only(mut self, drag_active: bool) -> Self {
        self.row = self.row.drop_only(drag_active);
        self
    }

    /// Configure drop-target behavior and hover-drop messages.
    pub fn drop_target_mode(mut self, drag_active: bool, hover_messages: bool) -> Self {
        self.row = self.row.drop_target_mode(drag_active, hover_messages);
        self
    }

    /// Mark whether a related row drag is active in this badge's container.
    pub fn drag_active(mut self, active: bool) -> Self {
        self.row = self.row.drag_active(active);
        self
    }

    /// Mark this badge as the source of the current container drag.
    pub fn drag_source(mut self, source: bool) -> Self {
        self.row = self.row.drag_source(source);
        self
    }

    /// Emit drag move messages while this badge remains the active drag source.
    pub fn drag_source_motion(mut self, enabled: bool) -> Self {
        self.row = self.row.drag_source_motion(enabled);
        self
    }

    /// Ignore hover updates while preserving activation and drag behavior.
    pub fn suppress_hover(mut self, suppress: bool) -> Self {
        self.row = self.row.suppress_hover(suppress);
        self
    }

    /// Clear retained hover state when synchronized from a previous tree.
    pub fn clear_hover_on_sync(mut self) -> Self {
        self.row = self.row.clear_hover_on_sync();
        self
    }

    /// Include primary-release modifier state in pointer activation messages.
    pub fn activation_modifiers(mut self) -> Self {
        self.row = self.row.activation_modifiers();
        self
    }

    /// Restrict pointer-motion routing to active badge interactions.
    pub fn pointer_motion_during_interaction(mut self) -> Self {
        self.row = self.row.pointer_motion_during_interaction();
        self
    }

    /// Mark app-owned interaction state that should keep pointer motion routed.
    pub fn pointer_motion_active(mut self, active: bool) -> Self {
        self.row = self.row.pointer_motion_active(active);
        self
    }

    /// Emit mapped host messages for badge interactions.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self { badge, row } = self;
        input_overlay(badge.passive_view(), row.mapped(map))
    }

    /// Emit host messages for selected badge interactions.
    pub fn filter_mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self { badge, row } = self;
        input_overlay(badge.passive_view(), row.filter_mapped(map))
    }
}

/// Build a badge or pill.
pub fn badge(label: impl Into<String>) -> BadgeBuilder {
    BadgeBuilder {
        label: PaintText::from(label.into()),
        style: None,
        active: false,
    }
}

/// Build an interactive badge or pill.
pub fn interactive_badge(label: impl Into<String>) -> InteractiveBadgeBuilder {
    badge(label).interactive()
}

/// Build a badge that emits one cloned host message when activated.
pub fn badge_message<Message>(label: impl Into<String>, message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    badge(label).message(message)
}

/// Build a badge with a custom widget-message mapper.
pub fn badge_mapped<Message: 'static>(
    label: impl Into<String>,
    map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    badge(label).mapped(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, app, column, text},
        gui::types::{Point, Rect},
        layout::Vector2,
        runtime::{PaintPrimitive, SurfaceRuntime, UiSurface},
        widgets::{PointerButton, PointerModifiers, WidgetInput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum DemoMessage {
        Activate,
        Secondary,
    }

    #[derive(Default)]
    struct DemoState {
        status: &'static str,
    }

    #[test]
    fn interactive_badge_routes_row_interactions_through_badge_visual() {
        let bridge = app(DemoState::default())
            .view(|state| {
                column([
                    interactive_badge("Tag")
                        .filter_mapped(|message| {
                            if message.secondary_position().is_some() {
                                return Some(DemoMessage::Secondary);
                            }
                            if message.is_single_activation() {
                                return Some(DemoMessage::Activate);
                            }
                            None
                        })
                        .width(80.0)
                        .height(22.0),
                    text(state.status).id(330).height(22.0),
                ])
                .spacing(0.0)
            })
            .update(|state, message| {
                state.status = match message {
                    DemoMessage::Activate => "activated",
                    DemoMessage::Secondary => "secondary",
                };
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 44.0));
        let position = Point::new(8.0, 8.0);

        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(
            runtime
                .surface()
                .find_widget(330)
                .and_then(|widget| widget
                    .widget_object()
                    .as_any()
                    .downcast_ref::<crate::widgets::TextWidget>())
                .map(|widget| widget.text.as_str()),
            Some("secondary")
        );
    }

    #[test]
    fn interactive_badge_paints_badge_content() {
        let frame = UiSurface::new(
            interactive_badge("Project")
                .primary()
                .active(true)
                .mapped(|_| ())
                .size(100.0, 22.0)
                .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 22.0)),
            &Default::default(),
        );

        let paints_label = frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Project"),
        );

        assert!(
            paints_label,
            "interactive badge should paint its badge label"
        );
    }

    #[test]
    fn badge_builder_passive_paints_without_host_message() {
        let frame = UiSurface::new(
            badge("Passive")
                .subtle()
                .passive::<()>()
                .size(100.0, 22.0)
                .into_node(),
        )
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 22.0)),
            &Default::default(),
        );

        let paints_label = frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Passive"),
        );

        assert!(paints_label, "passive badge should paint its label");
    }
}
