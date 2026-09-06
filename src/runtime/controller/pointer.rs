use super::focus::FocusTransition;
use super::interaction_state::{RuntimeManagedPointerCapture, RuntimeManagedPointerCaptureState};
use super::pointer_ingress::TypedPointerDeliveryContext;
use super::{PointerMoveOutcome, SurfaceRuntime};
use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    gui::types::Point,
    runtime::{CommandOutcome, NativeFileDrop, RuntimeBridge},
    widgets::{PointerButton, PointerPressAdmission},
    widgets::{PointerModifiers, WidgetId, WidgetInput},
};

pub(super) enum PointInputDispatch {
    Miss,
    Blocked,
    FocusVetoed,
    Routed(WidgetId, bool),
}

mod move_routing;
#[cfg(test)]
mod tests;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route pointer motion and return a redraw-oriented outcome for backend adapters.
    ///
    /// Use this in native or embedded backends that need to distinguish full
    /// scene rebuilds from paint-only runtime overlays. Application-level event
    /// routing can keep using [`Self::dispatch_event`].
    pub fn dispatch_pointer_move_with_outcome(&mut self, position: Point) -> PointerMoveOutcome {
        self.dispatch_pointer_move_with_refresh_outcome(
            position,
            true,
            PointerModifiers::default(),
            None,
            None,
        )
    }

    /// Route pointer motion while deferring host-surface refresh from emitted
    /// widget messages until the caller explicitly refreshes the runtime.
    ///
    /// Native backends use this during high-frequency pointer motion to
    /// coalesce many model updates into the next redraw instead of refreshing
    /// the declarative surface once per OS cursor event.
    pub fn dispatch_pointer_move_deferred_refresh_with_outcome(
        &mut self,
        position: Point,
    ) -> PointerMoveOutcome {
        self.dispatch_pointer_move_with_refresh_outcome(
            position,
            false,
            PointerModifiers::default(),
            None,
            None,
        )
    }

    pub(crate) fn dispatch_pointer_move_deferred_refresh_with_metadata(
        &mut self,
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> PointerMoveOutcome {
        self.dispatch_pointer_move_with_refresh_outcome(
            position,
            false,
            modifiers,
            timestamp,
            sequence_range,
        )
    }

    fn dispatch_pointer_move_with_refresh_outcome(
        &mut self,
        position: Point,
        refresh_after_message: bool,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> PointerMoveOutcome {
        let previous_hovered_widget = self.interaction.hover.widget;
        let previous_hovered_container = self.interaction.hover.container;
        let dispatch = self.dispatch_pointer_move_target_with_refresh_and_metadata(
            position,
            refresh_after_message,
            modifiers,
            timestamp,
            sequence_range,
        );
        self.service_pending_current_surface_relayout();
        let target = dispatch.target;
        let hover_changed = previous_hovered_widget != self.interaction.hover.widget
            || previous_hovered_container != self.interaction.hover.container;
        if hover_changed {
            self.clear_retained_hover_except(self.interaction.hover.widget);
        }
        let repaint_requested = self.take_repaint_requested();
        let exit_requested = self.take_exit_requested();
        let pointer_captured = self.interaction.pointer.capture.is_some();
        let target_can_paint_only = !hover_changed
            && target.is_some_and(|widget_id| self.widget_pointer_move_overlay_is_valid(widget_id));
        let drag_preview_can_paint_only =
            self.drag_preview_overlay_is_valid() && !hover_changed && !dispatch.emitted_output;
        let paint_only_requested = repaint_requested
            && !dispatch.emitted_output
            && (target_can_paint_only || drag_preview_can_paint_only);
        PointerMoveOutcome {
            target,
            hover_changed,
            pointer_captured,
            repaint_requested: repaint_requested && !paint_only_requested,
            paint_only_requested,
            exit_requested,
        }
    }

    pub(in crate::runtime::controller) fn reconcile_pointer_hover_after_capture_release(
        &mut self,
        position: Point,
    ) {
        self.reconcile_pointer_hover_state_without_input(position);
    }

    /// Route one normalized widget interaction by point hit test.
    ///
    /// Returns the targeted widget id when a projected widget handled the point.
    pub fn dispatch_input_at(&mut self, point: Point, input: WidgetInput) -> Option<WidgetId> {
        match self.dispatch_input_at_output(point, input) {
            PointInputDispatch::Routed(widget_id, _) => Some(widget_id),
            PointInputDispatch::Miss
            | PointInputDispatch::Blocked
            | PointInputDispatch::FocusVetoed => None,
        }
    }

    pub(super) fn dispatch_input_at_output(
        &mut self,
        point: Point,
        input: WidgetInput,
    ) -> PointInputDispatch {
        if self.gesture_owns_pointer_capture() {
            return PointInputDispatch::Blocked;
        }
        let Some(widget_id) = self.widget_at_for_input(point, &input) else {
            return PointInputDispatch::Miss;
        };
        let managed_press_compatibility_kind =
            self.pointer_press_target_compatibility_kind(widget_id);
        let admission = match &input {
            WidgetInput::PointerPress { .. } => {
                self.preflight_pointer_press_for_widget(widget_id, &input)
            }
            _ => PointerPressAdmission::Legacy,
        };
        let focus_press = matches!(
            &input,
            WidgetInput::PointerPress { .. } | WidgetInput::PointerDoubleClick { .. }
        );
        self.dispatch_input_at_target_output(
            widget_id,
            input,
            admission,
            false,
            focus_press,
            managed_press_compatibility_kind,
        )
    }

    pub(super) fn dispatch_input_at_target_output(
        &mut self,
        widget_id: WidgetId,
        input: WidgetInput,
        admission: PointerPressAdmission,
        install_legacy_capture: bool,
        focus_press: bool,
        managed_press_compatibility_kind: Option<&'static str>,
    ) -> PointInputDispatch {
        self.dispatch_input_at_target_output_with_delivery(
            widget_id,
            input,
            admission,
            install_legacy_capture,
            focus_press,
            managed_press_compatibility_kind,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn dispatch_input_at_target_output_with_delivery(
        &mut self,
        widget_id: WidgetId,
        input: WidgetInput,
        admission: PointerPressAdmission,
        install_legacy_capture: bool,
        focus_press: bool,
        managed_press_compatibility_kind: Option<&'static str>,
        delivery: Option<&mut TypedPointerDeliveryContext>,
    ) -> PointInputDispatch {
        if self.gesture_blocks_widget_input(&input) {
            return PointInputDispatch::Blocked;
        }
        let managed_press = match &input {
            WidgetInput::PointerPress { button, .. }
                if admission == PointerPressAdmission::ManagedCapture =>
            {
                Some(*button)
            }
            _ => None,
        };
        if admission == PointerPressAdmission::Blocked {
            return PointInputDispatch::Blocked;
        }
        self.validate_managed_pointer_capture_authority();
        if self.interaction.pointer.managed_capture.is_some()
            && (matches!(&input, WidgetInput::PointerPress { .. })
                || (focus_press && matches!(&input, WidgetInput::PointerDoubleClick { .. })))
        {
            return PointInputDispatch::Miss;
        }
        if install_legacy_capture
            && admission == PointerPressAdmission::Legacy
            && let WidgetInput::PointerPress { button, .. } = &input
        {
            self.interaction.pointer.capture = Some(widget_id);
            self.interaction.pointer.capture_button = Some(*button);
            self.reset_tooltip_hover_intent();
        }
        if focus_press
            && matches!(
                input,
                WidgetInput::PointerPress { .. } | WidgetInput::PointerDoubleClick { .. }
            )
        {
            let focus_transition = self.request_focus(widget_id);
            match focus_transition {
                FocusTransition::Vetoed => return PointInputDispatch::FocusVetoed,
                FocusTransition::InvalidTarget
                    if self
                        .surface_widget(widget_id)
                        .is_some_and(|widget| !widget.is_focusable()) =>
                {
                    let clear_transition = self.clear_focus_with_transition();
                    if clear_transition == FocusTransition::Vetoed {
                        return PointInputDispatch::FocusVetoed;
                    }
                }
                FocusTransition::InvalidTarget => {}
                FocusTransition::Unchanged | FocusTransition::Changed => {}
            }
        }
        if let Some(button) = managed_press
            && !self.reserve_managed_pointer_capture(
                widget_id,
                button,
                managed_press_compatibility_kind,
            )
        {
            return PointInputDispatch::Miss;
        }
        if let WidgetInput::PointerPress { button, .. }
        | WidgetInput::PointerDoubleClick { button, .. } = &input
        {
            self.clear_pointer_release_tombstone_for_new_press(*button);
        }
        let routed: Option<bool> = if let Some(delivery) = delivery
            && self.surface.widget_has_pointer_mapper(widget_id)
            && matches!(input, WidgetInput::PointerPress { .. })
        {
            self.issue_pointer_delivery(delivery)
                .ok()
                .map(|event| self.dispatch_pointer_output(widget_id, event))
        } else {
            self.dispatch_input_output(widget_id, input)
        };
        if let Some(button) = managed_press {
            self.finish_managed_pointer_press(widget_id, button, routed.is_some());
        }
        match routed {
            Some(emitted_output) => PointInputDispatch::Routed(widget_id, emitted_output),
            None => PointInputDispatch::Miss,
        }
    }

    pub(super) fn preflight_pointer_press_for_widget(
        &self,
        widget_id: WidgetId,
        input: &WidgetInput,
    ) -> PointerPressAdmission {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return PointerPressAdmission::Blocked;
        };
        self.surface_widget(widget_id)
            .map(|widget| widget.preflight_pointer_press(bounds, input))
            .unwrap_or(PointerPressAdmission::Blocked)
    }

    pub(super) fn pointer_press_target_compatibility_kind(
        &self,
        widget_id: WidgetId,
    ) -> Option<&'static str> {
        self.surface_widget(widget_id)
            .map(|widget| widget.compatibility_kind())
    }

    pub(super) fn dispatch_direct_input_output(
        &mut self,
        widget_id: WidgetId,
        input: WidgetInput,
    ) -> Option<bool> {
        let managed_press_compatibility_kind =
            self.pointer_press_target_compatibility_kind(widget_id);
        let admission = match &input {
            WidgetInput::PointerPress { .. } => {
                self.preflight_pointer_press_for_widget(widget_id, &input)
            }
            _ => PointerPressAdmission::Legacy,
        };
        let focus_press = admission == PointerPressAdmission::ManagedCapture
            && matches!(&input, WidgetInput::PointerPress { .. });
        match self.dispatch_input_at_target_output(
            widget_id,
            input,
            admission,
            false,
            focus_press,
            managed_press_compatibility_kind,
        ) {
            PointInputDispatch::Routed(_, emitted_output) => Some(emitted_output),
            PointInputDispatch::Miss
            | PointInputDispatch::Blocked
            | PointInputDispatch::FocusVetoed => None,
        }
    }

    fn managed_press_target_is_current(
        &self,
        widget_id: WidgetId,
        compatibility_kind: Option<&'static str>,
    ) -> bool {
        let Some(widget) = self.surface_widget(widget_id) else {
            return false;
        };
        let common = widget.widget_object().common();
        widget.id() == widget_id
            && compatibility_kind.is_none_or(|kind| widget.compatibility_kind() == kind)
            && !common.state.disabled
            && !common.state.read_only
            && (!widget.is_focusable()
                || self.interaction.focus.focused_widget() == Some(widget_id))
            && self.layout.rects.contains_key(&widget_id)
    }

    fn reserve_managed_pointer_capture(
        &mut self,
        widget_id: WidgetId,
        button: PointerButton,
        compatibility_kind: Option<&'static str>,
    ) -> bool {
        if self.interaction.pointer.managed_capture.is_some()
            || !self.managed_press_target_is_current(widget_id, compatibility_kind)
        {
            return false;
        }
        self.interaction.pointer.capture = None;
        self.interaction.pointer.capture_button = None;
        self.interaction.pointer.capture_state = None;
        self.interaction
            .pointer
            .set_release_tombstone(button, false);
        self.interaction.pointer.managed_capture = Some(RuntimeManagedPointerCapture {
            widget_id,
            button,
            state: RuntimeManagedPointerCaptureState::Pending,
        });
        true
    }

    fn finish_managed_pointer_press(
        &mut self,
        widget_id: WidgetId,
        button: PointerButton,
        dispatched: bool,
    ) {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return;
        };
        if capture.widget_id != widget_id
            || capture.button != button
            || capture.state != RuntimeManagedPointerCaptureState::Pending
        {
            return;
        }
        if !dispatched || !self.managed_press_target_is_current(widget_id, None) {
            self.terminate_managed_pointer_capture_without_cancel();
            return;
        }
        if !self.managed_pointer_record_is_live(false) {
            self.terminate_managed_pointer_capture_without_cancel();
            return;
        }
        self.interaction.pointer.managed_capture = Some(RuntimeManagedPointerCapture {
            widget_id,
            button,
            state: RuntimeManagedPointerCaptureState::Active,
        });
        self.interaction.pointer.capture = Some(widget_id);
        self.interaction.pointer.capture_button = Some(button);
        self.reset_tooltip_hover_intent();
        if self.validate_managed_pointer_capture_authority() {
            self.capture_pointer_capture_state(widget_id);
        }
    }

    pub(super) fn validate_managed_pointer_capture_authority(&mut self) -> bool {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return true;
        };
        if capture.state == RuntimeManagedPointerCaptureState::Cancelling {
            return false;
        }
        let require_shared_capture = capture.state == RuntimeManagedPointerCaptureState::Active;
        if self.managed_pointer_record_is_live(require_shared_capture) {
            true
        } else {
            self.terminate_managed_pointer_capture_without_cancel();
            false
        }
    }

    fn managed_pointer_record_is_live(&self, require_shared_capture: bool) -> bool {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return false;
        };
        if require_shared_capture && self.interaction.pointer.capture != Some(capture.widget_id) {
            return false;
        }
        let Some(widget) = self.surface_widget(capture.widget_id) else {
            return false;
        };
        let common = widget.widget_object().common();
        widget.id() == capture.widget_id
            && !common.state.disabled
            && !common.state.read_only
            && (!widget.is_focusable()
                || self.interaction.focus.focused_widget() == Some(capture.widget_id))
            && widget.retains_managed_pointer_capture()
    }

    pub(super) fn terminate_managed_pointer_capture_without_cancel(&mut self) {
        let Some(capture) = self.interaction.pointer.managed_capture.take() else {
            return;
        };
        if capture.state == RuntimeManagedPointerCaptureState::Cancelling {
            self.interaction.pointer.managed_capture = Some(capture);
            return;
        }
        if self.interaction.pointer.capture == Some(capture.widget_id) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
        self.interaction
            .pointer
            .set_release_tombstone(capture.button, true);
        self.reset_tooltip_hover_intent();
    }

    pub(super) fn terminate_managed_pointer_capture_for_widget(
        &mut self,
        widget_id: WidgetId,
    ) -> bool {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return false;
        };
        if capture.widget_id != widget_id {
            return false;
        }
        self.terminate_managed_pointer_capture_without_cancel();
        true
    }

    pub(super) fn clear_pointer_release_tombstone_for_new_press(&mut self, button: PointerButton) {
        self.interaction
            .pointer
            .set_release_tombstone(button, false);
    }

    pub(super) fn consume_pointer_release_tombstone(&mut self, button: PointerButton) -> bool {
        if !self.interaction.pointer.has_any_release_tombstone()
            || !self.interaction.pointer.has_release_tombstone(button)
        {
            return false;
        }
        self.interaction
            .pointer
            .set_release_tombstone(button, false);
        true
    }

    pub(super) fn managed_pointer_capture_for_button(
        &self,
        button: PointerButton,
    ) -> Option<WidgetId> {
        self.interaction
            .pointer
            .managed_capture
            .filter(|capture| {
                capture.state == RuntimeManagedPointerCaptureState::Active
                    && capture.button == button
            })
            .map(|capture| capture.widget_id)
    }

    pub(super) fn begin_managed_pointer_capture_cancellation(&mut self) -> Option<WidgetId> {
        let capture = self.interaction.pointer.managed_capture.as_mut()?;
        if matches!(capture.state, RuntimeManagedPointerCaptureState::Cancelling) {
            return None;
        }
        capture.state = RuntimeManagedPointerCaptureState::Cancelling;
        Some(capture.widget_id)
    }

    pub(super) fn finish_managed_pointer_capture_cancellation(&mut self) {
        let Some(capture) = self.interaction.pointer.managed_capture.take() else {
            return;
        };
        if capture.state != RuntimeManagedPointerCaptureState::Cancelling {
            self.interaction.pointer.managed_capture = Some(capture);
            return;
        }
        if self.interaction.pointer.capture == Some(capture.widget_id) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
        self.interaction
            .pointer
            .set_release_tombstone(capture.button, true);
        self.reset_tooltip_hover_intent();
    }

    pub(super) fn finish_managed_pointer_release(
        &mut self,
        widget_id: WidgetId,
        button: PointerButton,
    ) -> bool {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return false;
        };
        if capture.state != RuntimeManagedPointerCaptureState::Active
            || capture.widget_id != widget_id
            || capture.button != button
        {
            return false;
        }
        self.interaction.pointer.managed_capture = None;
        if self.interaction.pointer.capture == Some(widget_id) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
        true
    }

    pub(super) fn clear_managed_pointer_capture_for_widget(&mut self, widget_id: WidgetId) {
        if self
            .interaction
            .pointer
            .managed_capture
            .is_some_and(|capture| capture.widget_id == widget_id)
        {
            self.terminate_managed_pointer_capture_without_cancel();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reconcile_managed_pointer_capture_after_refresh(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        retired_widget_ids: &[WidgetId],
    ) {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return;
        };
        if capture.state == RuntimeManagedPointerCaptureState::Cancelling {
            return;
        }
        if retired_widget_ids.contains(&capture.widget_id)
            || !managed_capture_has_unique_widget_id(previous_widget_order, capture.widget_id)
            || !managed_capture_has_unique_widget_id(current_widget_order, capture.widget_id)
        {
            self.terminate_managed_pointer_capture_without_cancel();
            return;
        }
        let Some(previous_path) = previous_paths.get(&capture.widget_id) else {
            self.terminate_managed_pointer_capture_without_cancel();
            return;
        };
        let Some(current_path) = current_paths.get(&capture.widget_id) else {
            self.terminate_managed_pointer_capture_without_cancel();
            return;
        };
        let compatible = self
            .surface
            .widget_compatibility_at_path(previous_path.as_slice())
            .zip(next_surface.widget_compatibility_at_path(current_path.as_slice()))
            .is_some_and(
                |((previous_kind, previous_valid), (current_kind, current_valid))| {
                    previous_valid && current_valid && previous_kind == current_kind
                },
            );
        let previous_live = self
            .surface
            .find_widget_at_path(capture.widget_id, previous_path)
            .is_some_and(|widget| {
                self.managed_refresh_widget_is_live(widget, capture.widget_id, false)
            });
        let current_live = next_surface
            .find_widget_at_path(capture.widget_id, current_path)
            .is_some_and(|widget| {
                self.managed_refresh_widget_is_live(widget, capture.widget_id, false)
            });
        if !compatible || !previous_live || !current_live {
            self.terminate_managed_pointer_capture_without_cancel();
        }
    }

    fn managed_refresh_widget_is_live(
        &self,
        widget: &crate::runtime::SurfaceWidget<Message>,
        widget_id: WidgetId,
        require_shared_capture: bool,
    ) -> bool {
        let Some(capture) = self.interaction.pointer.managed_capture else {
            return false;
        };
        if require_shared_capture && self.interaction.pointer.capture != Some(capture.widget_id) {
            return false;
        }
        let common = widget.widget_object().common();
        widget.id() == widget_id
            && !common.state.disabled
            && !common.state.read_only
            && (!widget.is_focusable()
                || self.interaction.focus.focused_widget() == Some(widget_id))
            && widget.retains_managed_pointer_capture()
    }

    pub(super) fn unwind_provisional_pointer_capture(&mut self) {
        if self.interaction.pointer.managed_capture.is_some() {
            return;
        }
        self.interaction.pointer.capture = None;
        self.interaction.pointer.capture_button = None;
        self.interaction.pointer.capture_state = None;
        self.interaction.pointer.scroll_drag_capture = None;
        self.reset_tooltip_hover_intent();
    }

    /// Return whether a runtime-owned drag preview session is active.
    pub fn drag_session_active(&self) -> bool {
        self.interaction.drag.session.is_some()
    }

    /// Return whether the runtime-owned drag preview has a valid transient
    /// overlay for paint-only pointer presentation.
    ///
    /// Unlike a widget-local overlay, this validity evidence comes from the
    /// framework painter: a visible [`DragSession`](crate::runtime::DragSession)
    /// always has bounded preview primitives. It is kept separate from widget
    /// capability admission so the generic drag-preview authority remains
    /// explicit and testable.
    pub(crate) fn drag_preview_overlay_is_valid(&self) -> bool {
        self.interaction
            .drag
            .session
            .as_ref()
            .is_some_and(|session| session.visible)
    }

    /// Return the widget under a native file-drop pointer position.
    pub fn native_file_drop_target(&self, position: Option<Point>) -> Option<WidgetId> {
        position.and_then(|position| self.widget_at(position))
    }

    /// Route a native file-drop event to the topmost accepting declarative target.
    ///
    /// If no view-tree target accepts the drop, this falls back to the app-level
    /// native file-drop hook for compatibility with custom hosts.
    pub fn dispatch_native_file_drop(&mut self, drop: NativeFileDrop) -> CommandOutcome {
        if let Some(target) = self.native_file_drop_accepting_target(drop.position) {
            let drop = drop.clone().with_target_widget(Some(target));
            if let Some(message) = self.native_file_drop_message(target, drop.clone()) {
                return self.dispatch_message(message);
            }
            let command = self.host_native_file_drop(drop);
            return self.execute_command(command);
        }
        let target = self.native_file_drop_target(drop.position);
        let command = self.host_native_file_drop(drop.with_target_widget(target));
        self.execute_command(command)
    }

    fn native_file_drop_accepting_target(&self, position: Option<Point>) -> Option<WidgetId> {
        let Some(position) = position else {
            return self.topmost_visible_native_file_drop_target();
        };
        self.traversal
            .widgets
            .native_file_drop
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|widget_id| self.widget_contains_point(*widget_id, position))
    }

    fn topmost_visible_native_file_drop_target(&self) -> Option<WidgetId> {
        self.traversal
            .widgets
            .native_file_drop
            .visible()
            .iter()
            .rev()
            .copied()
            .next()
    }

    fn native_file_drop_message(
        &self,
        widget_id: WidgetId,
        drop: NativeFileDrop,
    ) -> Option<Message> {
        self.surface_widget(widget_id)
            .and_then(|widget| widget.dispatch_native_file_drop(widget_id, drop))
    }

    /// Clear active pointer capture without routing a release event.
    ///
    /// Native external drag loops own the release that completes the OS drag, so
    /// the originating surface must not keep treating later pointer motion as a
    /// continuation of the in-window press.
    pub(crate) fn cancel_pointer_capture(&mut self) {
        self.cancel_pointer_capture_with_delivery(None);
    }

    pub(in crate::runtime::controller) fn cancel_pointer_capture_with_delivery(
        &mut self,
        delivery: Option<crate::gui::pointer_ingress::PointerEvent>,
    ) -> bool {
        self.cancel_scroll_wheel_edit(true, None, true);
        self.cancel_gesture_capture(crate::widgets::GestureCancellation::CaptureLost);
        self.cancel_layout_pointer_capture();
        let managed_record_present = self.interaction.pointer.managed_capture.is_some();
        let managed_owner = self.begin_managed_pointer_capture_cancellation();
        let captured = self.interaction.pointer.capture.take();
        let captured_button = self.interaction.pointer.capture_button.take();
        let scroll_capture = self.interaction.pointer.scroll_drag_capture.take();
        let cancellation_owner = if managed_record_present {
            managed_owner
        } else {
            captured
        };
        let mut delivered = false;
        if let Some(widget_id) = cancellation_owner {
            if let Some(event) = delivery
                && self.surface.widget_has_pointer_mapper(widget_id)
            {
                let _ = self.dispatch_pointer_output(widget_id, event);
                delivered = true;
            } else {
                self.cancel_captured_widget_state(widget_id);
            }
        }
        if !managed_record_present
            && captured.is_some()
            && let Some(button) = captured_button
        {
            self.interaction.pointer.set_release_tombstone(button, true);
        }
        self.interaction.pointer.capture = None;
        self.interaction.pointer.capture_state = None;
        if let Some(capture) = scroll_capture {
            self.interaction
                .pointer
                .set_release_tombstone(capture.button, true);
            self.cancel_scrollbar_edit(capture, true);
        }
        if managed_record_present {
            self.finish_managed_pointer_capture_cancellation();
        }
        self.reset_tooltip_hover_intent();
        self.service_pending_current_surface_relayout();
        delivered
    }

    fn cancel_captured_widget_state(&mut self, widget_id: WidgetId) {
        let Some(previous_state) = self
            .surface_widget(widget_id)
            .map(|widget| widget.widget_object().common().state)
        else {
            return;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let result = self.dispatch_surface_pointer_capture_cancelled(widget_id, bounds);
        let Some(next_state) = self
            .surface_widget(widget_id)
            .map(|widget| widget.widget_object().common().state)
        else {
            return;
        };
        if previous_state != next_state {
            self.repaint_requested = true;
        }
        match result.map(|result| self.resolve_widget_dispatch(result)) {
            Some(crate::runtime::ResolvedWidgetDispatchResult::Message(message)) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
            }
            Some(crate::runtime::ResolvedWidgetDispatchResult::UnmappedOutput) => self.relayout(),
            Some(crate::runtime::ResolvedWidgetDispatchResult::NoOutput) | None => {}
        }
    }

    /// Clear pointer hover ownership and retained widget hover state.
    ///
    /// Native backends call this when the pointer leaves the surface or the
    /// window loses focus, because no later pointer-move event is guaranteed to
    /// arrive to clear paint-only hover fills.
    pub(crate) fn clear_pointer_hover(&mut self) -> bool {
        let mut cleared = false;
        if self.interaction.tooltip != Default::default() {
            self.reset_tooltip_hover_intent();
            cleared = true;
        }
        if let Some(widget_id) = self.interaction.hover.widget.take() {
            if let Some(widget) = self.surface.find_widget_mut(widget_id)
                && widget.widget_object().common().state.hovered
            {
                widget
                    .widget_object_mut_runtime()
                    .common_mut()
                    .state
                    .hovered = false;
            }
            cleared = true;
        }
        cleared |= self.clear_retained_hover_except(None);
        if self.interaction.hover.container.take().is_some() {
            cleared = true;
        }
        if self.interaction.hover.scroll_affordance.take().is_some() {
            self.note_scroll_visibility_mutation();
            cleared = true;
        }
        if self.interaction.hover.scroll_viewport.take().is_some() {
            self.note_scroll_visibility_mutation();
            cleared = true;
        }
        if cleared {
            self.repaint_requested = true;
        }
        cleared
    }

    pub(in crate::runtime::controller) fn clear_retained_hover_except(
        &mut self,
        owner: Option<WidgetId>,
    ) -> bool {
        let mut cleared = false;
        for index in 0..self.traversal.widgets.stateful_order.len() {
            let widget_id = self.traversal.widgets.stateful_order[index];
            if Some(widget_id) == owner {
                continue;
            }
            let Some(widget) = self.surface.find_widget_mut(widget_id) else {
                continue;
            };
            if !widget.widget_object().common().state.hovered {
                continue;
            }
            widget
                .widget_object_mut_runtime()
                .common_mut()
                .state
                .hovered = false;
            cleared = true;
        }
        if cleared {
            self.repaint_requested = true;
        }
        cleared
    }

    pub(in crate::runtime::controller) fn retain_hover_owner(&mut self, owner: Option<WidgetId>) {
        let Some(owner) = owner else {
            return;
        };
        let Some(widget) = self.surface.find_widget_mut(owner) else {
            return;
        };
        if widget.widget_object().common().state.hovered {
            return;
        }
        widget
            .widget_object_mut_runtime()
            .common_mut()
            .state
            .hovered = true;
        self.repaint_requested = true;
    }

    /// End the runtime drag preview because ownership has moved to a native
    /// external drag loop.
    pub(crate) fn take_drag_preview_for_external_drag(&mut self) -> bool {
        if self.interaction.drag.session.take().is_none() {
            return false;
        }
        self.repaint_requested = true;
        true
    }

    /// Hide the runtime drag preview while the pointer is outside this surface.
    ///
    /// The drag session stays alive so a later pointer move back into the
    /// window can show the preview again and continue routing the same drag.
    pub(crate) fn hide_drag_preview_for_cursor_left(&mut self) -> bool {
        let Some(session) = self.interaction.drag.session.as_mut() else {
            return false;
        };
        if !session.visible {
            return false;
        }
        session.visible = false;
        self.repaint_requested = true;
        true
    }
}

fn managed_capture_has_unique_widget_id(widget_order: &[WidgetId], widget_id: WidgetId) -> bool {
    let mut found = false;
    for candidate in widget_order {
        if *candidate != widget_id {
            continue;
        }
        if found {
            return false;
        }
        found = true;
    }
    found
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PointerMoveDispatch {
    pub(super) target: Option<WidgetId>,
    pub(super) emitted_output: bool,
}
