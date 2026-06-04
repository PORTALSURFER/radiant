//! Reusable dense-list/tree row interaction primitive.

use crate::gui::{
    list::{DenseRowPalette, DenseRowVisualState, push_dense_row_fill},
    types::Rect,
};
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    DragHandleMessage, InteractiveRowMessage, PointerModifiers, WidgetInput, WidgetOutput,
};
use crate::widgets::primitives::support::{WidgetCommon, push_control_chrome};
use std::sync::Arc;

mod builders;
mod input;

#[cfg(test)]
mod tests;

/// Public interactive row primitive for selectable, draggable, droppable rows.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveRowWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable row configuration.
    pub props: InteractiveRowProps,
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

/// Host callbacks for common interactive-row message routing.
///
/// Use this router when a row host only needs the standard activation,
/// secondary-click, drag, drop, and hover-drop interaction shapes translated
/// into its own message type.
#[derive(Clone)]
pub struct InteractiveRowActions<Message> {
    activate: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    activate_with_modifiers:
        Option<Arc<dyn Fn(PointerModifiers) -> Message + Send + Sync + 'static>>,
    double_activate: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    secondary: Option<Arc<dyn Fn(crate::gui::types::Point) -> Message + Send + Sync + 'static>>,
    drag: Option<Arc<dyn Fn(DragHandleMessage) -> Message + Send + Sync + 'static>>,
    drop: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    hover_drop: Option<Arc<dyn Fn(crate::gui::types::Point) -> Message + Send + Sync + 'static>>,
}

impl<Message> InteractiveRowActions<Message> {
    /// Build an empty row-action router.
    pub fn new() -> Self {
        Self {
            activate: None,
            activate_with_modifiers: None,
            double_activate: None,
            secondary: None,
            drag: None,
            drop: None,
            hover_drop: None,
        }
    }

    /// Emit a host message for single primary activation.
    pub fn activate(mut self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.activate = Some(Arc::new(message));
        self
    }

    /// Emit a host message for single primary activation with modifier state.
    pub fn activate_with_modifiers(
        mut self,
        message: impl Fn(PointerModifiers) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.activate_with_modifiers = Some(Arc::new(message));
        self
    }

    /// Emit a host message for double primary activation.
    pub fn double_activate(
        mut self,
        message: impl Fn() -> Message + Send + Sync + 'static,
    ) -> Self {
        self.double_activate = Some(Arc::new(message));
        self
    }

    /// Emit a host message for secondary activation.
    pub fn secondary(
        mut self,
        message: impl Fn(crate::gui::types::Point) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.secondary = Some(Arc::new(message));
        self
    }

    /// Emit a host message for drag lifecycle updates.
    pub fn drag(
        mut self,
        message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.drag = Some(Arc::new(message));
        self
    }

    /// Emit a host message when a drop lands on the row.
    pub fn drop(mut self, message: impl Fn() -> Message + Send + Sync + 'static) -> Self {
        self.drop = Some(Arc::new(message));
        self
    }

    /// Emit a host message when another row drag hovers this drop target.
    pub fn hover_drop(
        mut self,
        message: impl Fn(crate::gui::types::Point) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.hover_drop = Some(Arc::new(message));
        self
    }

    /// Emit drop and hover-drop messages for one host-owned target key.
    ///
    /// Use this when several row-like surfaces map drop and hover-drop to the
    /// same durable application key, such as a folder, category, layer, lane, or
    /// collection. The key stays host-owned; Radiant only centralizes the common
    /// row-action routing shape.
    pub fn drop_target_key<Key>(
        mut self,
        key: Key,
        drop_message: impl Fn(Key) -> Message + Send + Sync + 'static,
        hover_drop_message: impl Fn(Key, crate::gui::types::Point) -> Message + Send + Sync + 'static,
    ) -> Self
    where
        Key: Clone + Send + Sync + 'static,
    {
        let hover_key = key.clone();
        self.drop = Some(Arc::new(move || drop_message(key.clone())));
        self.hover_drop = Some(Arc::new(move |position| {
            hover_drop_message(hover_key.clone(), position)
        }));
        self
    }

    /// Route a generic row interaction into the configured host action.
    pub fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        if let Some(position) = message.secondary_position() {
            return self.secondary.as_ref().map(|callback| callback(position));
        }
        if let Some(drag) = message.drag_message() {
            return self.drag.as_ref().map(|callback| callback(drag));
        }
        if message.is_drop() {
            return self.drop.as_ref().map(|callback| callback());
        }
        if let Some(position) = message.hover_drop_position() {
            return self.hover_drop.as_ref().map(|callback| callback(position));
        }
        if message.is_double_activation() {
            return self.double_activate.as_ref().map(|callback| callback());
        }
        if let Some(modifiers) = message.single_activation_modifiers() {
            if let Some(callback) = &self.activate_with_modifiers {
                return Some(callback(modifiers));
            }
            return self.activate.as_ref().map(|callback| callback());
        }
        None
    }
}

impl<Message> Default for InteractiveRowActions<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> std::fmt::Debug for InteractiveRowActions<Message> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InteractiveRowActions")
            .finish_non_exhaustive()
    }
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

    /// Project this interactive row into generic dense-list visual state.
    pub fn dense_visual_state(&self, parts: InteractiveRowVisualStateParts) -> DenseRowVisualState {
        DenseRowVisualState {
            selected: parts.selected,
            hovered: self.common.state.hovered,
            pressed: self.common.state.pressed,
            active_target: parts.active_target,
            candidate: parts.candidate,
        }
    }

    /// Push this row's highest-priority dense feedback fill into a custom paint plan.
    ///
    /// Custom row wrappers can use this when they compose an
    /// `InteractiveRowWidget` for retained hover, pressed, drag, and drop state
    /// but paint their own row content.
    pub fn push_dense_fill(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        parts: InteractiveRowVisualStateParts,
        palette: DenseRowPalette,
    ) -> bool {
        push_dense_row_fill(
            primitives,
            self.id(),
            bounds,
            self.dense_visual_state(parts),
            palette,
        )
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

/// Custom widget contract for widgets built around an embedded interactive row.
///
/// Implement this trait when a custom-painted row needs Radiant's generic row
/// input, pointer-motion policy, retained state synchronization, and widget
/// contract delegation, but the host still owns the row's visual content and
/// message type. The blanket [`Widget`] implementation keeps application row
/// wrappers focused on domain action routing and paint.
pub trait EmbeddedInteractiveRowWidget: Clone + Send + Sync + 'static {
    /// Host-specific message emitted by the custom row.
    type Message: Send + Sync + 'static;

    /// Return the embedded generic interactive row.
    fn interactive_row(&self) -> &InteractiveRowWidget;

    /// Return the embedded generic interactive row mutably.
    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget;

    /// Return common action routing for this embedded row, when applicable.
    fn interactive_row_actions(&self) -> Option<&InteractiveRowActions<Self::Message>> {
        None
    }

    /// Map a generic row interaction into this custom row's message type.
    fn map_interactive_row_message(&self, message: InteractiveRowMessage) -> Option<Self::Message> {
        self.interactive_row_actions()?.route(message)
    }

    /// Append host-specific paint for this custom row.
    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    );
}

impl<T> Widget for T
where
    T: EmbeddedInteractiveRowWidget,
{
    fn common(&self) -> &WidgetCommon {
        self.interactive_row().common()
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        self.interactive_row_mut().common_mut()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let message = self.interactive_row_mut().handle_input(bounds, input)?;
        self.map_interactive_row_message(message)
            .map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        self.interactive_row().accepts_pointer_move()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<T>() else {
            return;
        };
        self.interactive_row_mut()
            .synchronize_from_previous(previous.interactive_row());
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.append_interactive_row_paint(primitives, bounds, layout, theme);
    }
}

impl Widget for InteractiveRowWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        InteractiveRowWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        match self.props.pointer_motion {
            InteractiveRowPointerMotion::Always => true,
            InteractiveRowPointerMotion::DuringInteraction => {
                self.common.state.pressed
                    || self.props.drag_active
                    || self.props.drag_source
                    || self.props.pointer_motion_active
            }
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            if self.props.clear_hover_on_sync {
                self.common.state.hovered = false;
            }
            self.dragged = previous.dragged;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        if self.common.paint.paints_state_layers {
            push_control_chrome(primitives, &self.common, bounds, theme);
        }
    }
}
