use crate::{
    application::{ViewNode, input_overlay, interactive_row},
    widgets::{FocusBehavior, InteractiveRowMessage, PaintBounds, WidgetSizing, WidgetStyle},
};

use super::{BadgeBuilder, badge};
use crate::application::control_builders::interactive_row::{
    InteractiveRowActions, InteractiveRowBuilder,
};

/// Builder for badge visuals with rich row-style interactions.
pub struct InteractiveBadgeBuilder {
    badge: BadgeBuilder,
    row: InteractiveRowBuilder,
}

impl BadgeBuilder {
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

    /// Configure this badge as a host-tracked drag source.
    ///
    /// This preset keeps pointer-motion routing active while a badge
    /// interaction is in progress, marks whether a related badge or row drag is
    /// active in the host container, and records whether this badge is the
    /// active drag source.
    pub fn tracked_drag_source(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.row = self.row.tracked_drag_source(drag_active, drag_source);
        self
    }

    /// Configure this badge as a host-tracked drag source that emits retained
    /// source move messages.
    ///
    /// Use this when the active source may be rebuilt from host state during a
    /// drag and should continue reporting pointer movement after projection.
    pub fn tracked_drag_source_with_motion(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.row = self
            .row
            .tracked_drag_source_with_motion(drag_active, drag_source);
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

    /// Configure a host-tracked badge drop target.
    ///
    /// While `active_target` is true, the badge still accepts the eventual drop
    /// but suppresses duplicate hover-drop messages and keeps pointer-motion
    /// routing active through the host-owned interaction state.
    pub fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.row = self.row.tracked_drop_target(drag_active, active_target);
        self
    }

    /// Configure a host-tracked conditional badge drop target.
    ///
    /// Use this when host-owned validation decides whether this badge is a
    /// valid drop target, but Radiant should still route target-enter and
    /// stale-target clear lifecycle messages through the badge's interaction
    /// layer.
    pub fn tracked_drop_candidate(
        mut self,
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    ) -> Self {
        self.row =
            self.row
                .tracked_drop_candidate(drag_active, current_target, candidate, active_target);
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

    /// Emit host messages for common badge row actions.
    pub fn actions<Message: 'static>(
        self,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        let Self { badge, row } = self;
        input_overlay(badge.passive_view(), row.actions(actions))
    }
}

/// Build an interactive badge or pill.
pub fn interactive_badge(
    label: impl Into<crate::application::TextContent>,
) -> InteractiveBadgeBuilder {
    badge(label).interactive()
}
