//! Reusable dense-list/tree row interaction primitive.

use crate::gui::types::{Point, Rect};
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{InteractiveRowMessage, WidgetInput, WidgetOutput};
use crate::widgets::primitives::support::WidgetCommon;

mod actions;
mod builders;
mod embedded;
mod input;
mod paint;
mod widget_impl;

pub use actions::InteractiveRowActions;
pub use embedded::EmbeddedInteractiveRowWidget;

#[cfg(test)]
mod tests;

/// Public interactive row primitive for selectable, draggable, droppable rows.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveRowWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable row configuration.
    pub props: InteractiveRowProps,
    pressed_position: Option<Point>,
    dragged: bool,
}

/// Immutable interactive row configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InteractiveRowProps {
    /// Emit drag lifecycle messages after pointer movement while pressed.
    pub draggable: bool,
    /// Emit drop and hover-drop-target messages.
    pub droppable: bool,
    /// Whether another row drag is currently active in this container.
    pub drag_active: bool,
    /// Whether this row is the source of the active drag in this container.
    pub drag_source: bool,
    /// Whether active drag-source rows emit move messages from pointer motion.
    pub drag_source_motion: bool,
    /// Whether pointer hover should be ignored and cleared for this row.
    pub suppress_hover: bool,
    /// Whether active drop-target hover emits hover messages.
    pub drop_hover: bool,
    /// Clear stale hover state when a retained row is synchronized.
    pub clear_hover_on_sync: bool,
    /// Emit modifier-aware activation messages for primary pointer release.
    pub activation_modifiers: bool,
    /// Pointer-motion routing policy for this row.
    pub pointer_motion: InteractiveRowPointerMotion,
    /// Extra app-owned activity that should keep pointer motion routed.
    pub pointer_motion_active: bool,
}

/// Host-owned dense-row state that can be merged with interactive row state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InteractiveRowVisualStateParts {
    /// The row is selected by the host application.
    pub selected: bool,
    /// The row is the committed target for an active operation.
    pub active_target: bool,
    /// The row is a valid candidate for an active operation.
    pub candidate: bool,
}

/// Pointer-motion routing policy for dense interactive rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractiveRowPointerMotion {
    /// Always receive pointer motion, allowing normal hover updates.
    #[default]
    Always,
    /// Receive pointer motion only while pressed, dragging, dropping, or app-active.
    DuringInteraction,
}

/// Named construction fields for [`InteractiveRowWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveRowWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Intrinsic interactive-row sizing contract.
    pub sizing: WidgetSizing,
}

impl InteractiveRowWidget {
    /// Build an interactive row descriptor from named identity and sizing fields.
    pub fn from_parts(parts: InteractiveRowWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        Self {
            common,
            props: InteractiveRowProps::default(),
            pressed_position: None,
            dragged: false,
        }
    }

    /// Build an interactive row descriptor.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::from_parts(InteractiveRowWidgetParts { id, sizing })
    }

    /// Stable widget identity used by layout, input, retained state, and paint.
    pub fn id(&self) -> WidgetId {
        self.common.id
    }

    /// Shared widget contract for custom row wrappers.
    pub fn common(&self) -> &WidgetCommon {
        &self.common
    }

    /// Mutable shared widget contract for custom row wrappers.
    pub fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    /// Enable drag lifecycle messages.
    pub fn with_drag(mut self) -> Self {
        self.props.draggable = true;
        self
    }

    /// Enable drop and drop-hover messages.
    pub fn with_drop_target(mut self, drag_active: bool) -> Self {
        self.props.droppable = true;
        self.props.drag_active = drag_active;
        self.props.drop_hover = true;
        self
    }

    /// Enable drop handling without hover-drop notifications.
    pub fn with_drop_only(mut self, drag_active: bool) -> Self {
        self.props.droppable = true;
        self.props.drag_active = drag_active;
        self.props.drop_hover = false;
        self
    }

    /// Configure drop handling and whether hover-drop notifications are emitted.
    ///
    /// When `drag_active` is false, this leaves the row out of the drop-target
    /// path. When it is true, `hover_messages` controls whether pointer motion
    /// over the row emits hover-drop messages or only accepts the eventual drop.
    pub fn with_drop_target_mode(mut self, drag_active: bool, hover_messages: bool) -> Self {
        self.props.droppable = drag_active;
        self.props.drop_hover = drag_active && hover_messages;
        self.props.drag_active = drag_active;
        self
    }

    /// Configure host-tracked conditional drop behavior.
    ///
    /// `candidate` is host-owned validation for this row. `active_target`
    /// keeps pointer motion routed while the host has a committed target, so a
    /// non-candidate row can clear that target. `current_target` suppresses
    /// duplicate hover-drop messages while still allowing the final drop.
    pub fn with_tracked_drop_candidate(
        mut self,
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    ) -> Self {
        self.props.droppable = drag_active;
        self.props.drop_hover = drag_active && !current_target && (candidate || active_target);
        self.props.drag_active = drag_active;
        self.props.pointer_motion = InteractiveRowPointerMotion::DuringInteraction;
        self.props.pointer_motion_active = active_target;
        self
    }

    /// Mark whether a related row drag is currently active in the same container.
    pub fn with_drag_active(mut self, drag_active: bool) -> Self {
        self.props.drag_active = drag_active;
        self
    }

    /// Mark this row as the source of the current external/container drag.
    pub fn with_drag_source(mut self, drag_source: bool) -> Self {
        self.props.drag_source = drag_source;
        self
    }

    /// Emit move messages while this row is already the active drag source.
    pub fn with_drag_source_motion(mut self, enabled: bool) -> Self {
        self.props.drag_source_motion = enabled;
        self
    }

    /// Ignore pointer hover for this row while preserving other interactions.
    pub fn suppress_hover(mut self, suppress_hover: bool) -> Self {
        self.props.suppress_hover = suppress_hover;
        self
    }

    /// Clear retained hover during widget synchronization.
    pub fn clear_hover_on_sync(mut self) -> Self {
        self.props.clear_hover_on_sync = true;
        self
    }

    /// Include primary-release modifier state in pointer activation messages.
    pub fn with_activation_modifiers(mut self) -> Self {
        self.props.activation_modifiers = true;
        self
    }

    /// Restrict pointer-motion routing to active interactions.
    pub fn with_pointer_motion_during_interaction(mut self) -> Self {
        self.props.pointer_motion = InteractiveRowPointerMotion::DuringInteraction;
        self
    }

    /// Mark app-owned interaction state that should keep pointer motion routed.
    pub fn with_pointer_motion_active(mut self, active: bool) -> Self {
        self.props.pointer_motion_active = active;
        self
    }

    /// Route input through this row and map the row message into a typed widget output.
    ///
    /// Custom-painted row widgets can use this when they compose an
    /// `InteractiveRowWidget` for generic row behavior but expose a
    /// host-specific message type from their own [`Widget`] implementation.
    pub fn handle_input_mapped<Message: Send + Sync + 'static>(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
        map: impl FnOnce(InteractiveRowMessage) -> Option<Message>,
    ) -> Option<WidgetOutput> {
        self.handle_input(bounds, input)
            .and_then(map)
            .map(WidgetOutput::typed)
    }

    /// Synchronize this embedded row from a previous custom widget instance.
    ///
    /// Returns `true` when `previous` had the expected concrete type and the
    /// embedded row state was synchronized. Returns `false` when the previous
    /// retained widget had another type.
    pub fn synchronize_from_previous_embedded<Host: 'static>(
        &mut self,
        previous: &dyn Widget,
        previous_row: impl FnOnce(&Host) -> &InteractiveRowWidget,
    ) -> bool {
        let Some(previous) = previous.as_any().downcast_ref::<Host>() else {
            return false;
        };
        self.synchronize_from_previous(previous_row(previous));
        true
    }
}
