use std::collections::HashMap;

use super::{FocusTraversal, SurfaceRuntime};
use crate::widgets::FocusLossDecision;
use crate::{
    gui::input::InputTimestamp,
    gui::{focus::FocusSurface, input::KeyPress},
    runtime::RuntimeBridge,
    widgets::{WidgetId, WidgetInput, WidgetKey},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusTransition {
    InvalidTarget,
    Vetoed,
    Unchanged,
    Changed,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Give keyboard focus to one focusable widget.
    ///
    /// Returns `false` when the widget is absent or does not participate in
    /// focus. A valid target may retain the current owner when that widget
    /// vetoes focus loss. Focus changes are routed into affected widgets so
    /// their retained interaction state can update before the next paint plan.
    pub fn focus_widget(&mut self, widget_id: WidgetId) -> bool {
        !matches!(
            self.request_focus(widget_id),
            FocusTransition::InvalidTarget
        )
    }

    /// Clear keyboard focus when a surface or backend loses focus ownership.
    pub fn clear_focus(&mut self) {
        let _ = self.clear_focus_with_transition();
    }

    pub(super) fn clear_focus_with_transition(&mut self) -> FocusTransition {
        let Some(previous) = self.interaction.focus.focused_widget else {
            return FocusTransition::Unchanged;
        };
        let previous_is_live = self.focus_owner_can_prepare_loss(previous);
        if previous_is_live && self.prepare_focus_loss(previous) == FocusLossDecision::Veto {
            self.repaint_requested = true;
            return FocusTransition::Vetoed;
        }
        self.interaction.focus.focused_widget = None;
        if previous_is_live {
            self.route_focus_changed(previous, false);
        }
        FocusTransition::Changed
    }

    pub(super) fn request_focus(&mut self, widget_id: WidgetId) -> FocusTransition {
        if !self.is_live_focus_target(widget_id) {
            return FocusTransition::InvalidTarget;
        }
        if self.interaction.focus.focused_widget == Some(widget_id) {
            return FocusTransition::Unchanged;
        }

        if let Some(previous) = self.interaction.focus.focused_widget {
            let previous_is_live = self.focus_owner_can_prepare_loss(previous);
            if previous_is_live && self.prepare_focus_loss(previous) == FocusLossDecision::Veto {
                self.repaint_requested = true;
                return FocusTransition::Vetoed;
            }

            // Install the controller-owned target before FocusChanged(false)
            // can emit a message and synchronously reproject the surface.
            self.interaction.focus.focused_widget = Some(widget_id);
            let application_projection_before = self.refresh_counters().application_projection;
            if previous_is_live {
                self.route_focus_changed(previous, false);
            }
            let reprojected =
                self.refresh_counters().application_projection != application_projection_before;
            if !reprojected && self.is_authoritative_focus_target(widget_id) {
                self.route_focus_changed(widget_id, true);
            }
        } else {
            self.interaction.focus.focused_widget = Some(widget_id);
            if self.is_authoritative_focus_target(widget_id) {
                self.route_focus_changed(widget_id, true);
            }
        }
        if self.interaction.focus.focused_widget == Some(widget_id)
            && !self.is_authoritative_focus_target(widget_id)
        {
            // A focus-loss output may remove or supersede the proposed target
            // while the old owner is being routed out.
            self.interaction.focus.focused_widget = None;
            return FocusTransition::InvalidTarget;
        }
        if self.interaction.focus.focused_widget == Some(widget_id) {
            FocusTransition::Changed
        } else {
            FocusTransition::InvalidTarget
        }
    }

    pub(super) fn is_live_focus_target(&self, widget_id: WidgetId) -> bool {
        self.traversal
            .widgets
            .paths
            .current
            .contains_key(&widget_id)
            && self.traversal.widgets.focusable.contains(widget_id)
            && self.layout.rects.contains_key(&widget_id)
            && self
                .surface_widget(widget_id)
                .is_some_and(|widget| widget.is_focusable())
    }

    pub(super) fn is_authoritative_focus_target(&self, widget_id: WidgetId) -> bool {
        self.interaction.focus.focused_widget == Some(widget_id)
            && self.is_live_focus_target(widget_id)
    }

    fn focus_owner_can_prepare_loss(&self, widget_id: WidgetId) -> bool {
        self.is_live_focus_target(widget_id)
    }

    fn prepare_focus_loss(&mut self, widget_id: WidgetId) -> FocusLossDecision {
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return FocusLossDecision::Allow;
        };
        self.surface
            .find_widget_mut_at_path(widget_id, child_path)
            .map(|widget| widget.widget_object_mut_runtime().prepare_focus_loss())
            .unwrap_or(FocusLossDecision::Allow)
    }

    /// Move keyboard focus through the current declarative tree.
    ///
    /// Traversal uses stable tree order and wraps at either end. Returns the new
    /// focus target, or `None` when no keyboard-focusable widgets are projected.
    pub fn traverse_focus(&mut self, direction: FocusTraversal) -> Option<WidgetId> {
        let next = next_focus_target(
            self.interaction.focus.focused_widget,
            self.traversal.widgets.keyboard_focus.order(),
            self.traversal.widgets.keyboard_focus.rank(),
            direction,
        )?;
        match self.request_focus(next) {
            FocusTransition::Changed | FocusTransition::Unchanged => Some(next),
            FocusTransition::InvalidTarget | FocusTransition::Vetoed => None,
        }
    }

    /// Route a keyboard interaction to the current focus target.
    ///
    /// Pointer events should continue to use [`SurfaceRuntime::dispatch_input_at`]
    /// or [`SurfaceRuntime::dispatch_input`], because they carry their own hit
    /// target. Keyboard events are resolved through focused widget identity.
    pub fn dispatch_focused_input(&mut self, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.interaction.focus.focused_widget?;
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    /// Return whether the current focus target is a text input.
    pub fn focused_text_input_id(&self) -> Option<WidgetId> {
        let widget_id = self.interaction.focus.focused_widget?;
        self.surface_widget(widget_id).and_then(|widget| {
            widget
                .widget_object()
                .accepts_text_input()
                .then_some(widget_id)
        })
    }

    /// Return whether the focused widget asks to receive `key` before host shortcuts.
    pub fn focused_widget_preempts_host_shortcut_key(&self, key: WidgetKey) -> bool {
        let Some(widget_id) = self.interaction.focus.focused_widget else {
            return false;
        };
        self.surface_widget(widget_id)
            .is_some_and(|widget| widget.widget_object().preempts_host_shortcut_key(key))
    }

    /// Return selected text from the focused text input as a borrowed slice, if any.
    pub fn focused_text_selection_slice(&self) -> Option<&str> {
        let widget_id = self.focused_text_input_id()?;
        self.surface_widget(widget_id)
            .and_then(|widget| widget.widget_object().selected_text_slice())
    }

    /// Return selected text from the focused text input as an owned string, if any.
    pub fn focused_text_selection(&self) -> Option<String> {
        self.focused_text_selection_slice().map(str::to_owned)
    }

    /// Resolve one keypress through host-owned shortcuts before falling back to
    /// focused-widget key routing.
    ///
    /// Returns `true` when the shortcut catalog handled the press or a focused
    /// widget received it.
    pub fn dispatch_key_press(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
        focus: FocusSurface,
    ) -> bool {
        self.dispatch_key_press_with_timestamp(press, widget_key, focus, None)
    }

    /// Resolve one keypress and preserve an optional native input timestamp on
    /// focused-widget fallback routing.
    pub(crate) fn dispatch_key_press_with_timestamp(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
        focus: FocusSurface,
        timestamp: Option<InputTimestamp>,
    ) -> bool {
        let resolution =
            self.host_resolve_key_press(self.interaction.focus.pending_key_chord, press, focus);
        self.interaction.focus.pending_key_chord = resolution.pending_chord;
        if let Some(message) = resolution.action {
            let outcome = self.dispatch_message(message);
            self.pending_input_command_outcome.merge(outcome);
            return true;
        }
        if resolution.handled {
            return true;
        }
        widget_key
            .and_then(|key| {
                self.dispatch_focused_input(WidgetInput::key_press_with_timestamp(key, timestamp))
            })
            .is_some()
    }

    pub(super) fn route_focus_changed(&mut self, widget_id: WidgetId, focused: bool) {
        let _ = self.dispatch_input_output(widget_id, WidgetInput::FocusChanged(focused));
    }

    pub(super) fn restore_focused_widget_state(&mut self, widget_id: WidgetId) {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let _ = self.dispatch_surface_input(widget_id, bounds, WidgetInput::FocusChanged(true));
    }
}

fn next_focus_target(
    current: Option<WidgetId>,
    order: &[WidgetId],
    rank: &HashMap<WidgetId, usize>,
    direction: FocusTraversal,
) -> Option<WidgetId> {
    if order.is_empty() {
        return None;
    }
    let current_index = current.and_then(|widget_id| rank.get(&widget_id).copied());
    let next_index = match (current_index, direction) {
        (Some(index), FocusTraversal::Forward) => (index + 1) % order.len(),
        (Some(0), FocusTraversal::Backward) => order.len() - 1,
        (Some(index), FocusTraversal::Backward) => index - 1,
        (None, FocusTraversal::Forward) => 0,
        (None, FocusTraversal::Backward) => order.len() - 1,
    };
    Some(order[next_index])
}
