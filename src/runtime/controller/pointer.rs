use super::{PointerMoveOutcome, SurfaceRuntime};
use crate::{
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{WidgetId, WidgetInput},
};

mod move_routing;

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
        self.dispatch_pointer_move_with_refresh_outcome(position, true)
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
        self.dispatch_pointer_move_with_refresh_outcome(position, false)
    }

    fn dispatch_pointer_move_with_refresh_outcome(
        &mut self,
        position: Point,
        refresh_after_message: bool,
    ) -> PointerMoveOutcome {
        let previous_hovered_widget = self.interaction.hover.widget;
        let previous_hovered_container = self.interaction.hover.container;
        let dispatch =
            self.dispatch_pointer_move_target_with_refresh(position, refresh_after_message);
        let target = dispatch.target;
        let repaint_requested = self.take_repaint_requested();
        let exit_requested = self.take_exit_requested();
        let hover_changed = previous_hovered_widget != self.interaction.hover.widget
            || previous_hovered_container != self.interaction.hover.container;
        let pointer_captured = self.interaction.pointer.capture.is_some();
        let target_prefers_paint_only =
            target.is_some_and(|widget_id| self.widget_prefers_pointer_move_paint_only(widget_id));
        let drag_preview_can_paint_only =
            self.drag_session_active() && !hover_changed && !dispatch.emitted_output;
        let paint_only_requested = repaint_requested
            && !dispatch.emitted_output
            && (target_prefers_paint_only || drag_preview_can_paint_only);
        PointerMoveOutcome {
            target,
            hover_changed,
            pointer_captured,
            repaint_requested: repaint_requested && !paint_only_requested,
            paint_only_requested,
            exit_requested,
        }
    }

    /// Route one normalized widget interaction by point hit test.
    ///
    /// Returns the targeted widget id when a projected widget handled the point.
    pub fn dispatch_input_at(&mut self, point: Point, input: WidgetInput) -> Option<WidgetId> {
        self.dispatch_input_at_output(point, input)
            .map(|(widget_id, _)| widget_id)
    }

    pub(super) fn dispatch_input_at_output(
        &mut self,
        point: Point,
        input: WidgetInput,
    ) -> Option<(WidgetId, bool)> {
        let widget_id = self.widget_at(point)?;
        if matches!(
            input,
            WidgetInput::PointerPress { .. } | WidgetInput::PointerDoubleClick { .. }
        ) && !self.focus_widget(widget_id)
        {
            self.clear_focus();
        }
        self.dispatch_input_output(widget_id, input)
            .map(|emitted_output| (widget_id, emitted_output))
    }

    /// Return whether a runtime-owned drag preview session is active.
    pub fn drag_session_active(&self) -> bool {
        self.interaction.drag.session.is_some()
    }

    /// Return the widget under a native file-drop pointer position.
    pub fn native_file_drop_target(&self, position: Option<Point>) -> Option<WidgetId> {
        position.and_then(|position| self.widget_at(position))
    }

    /// Clear active pointer capture without routing a release event.
    ///
    /// Native external drag loops own the release that completes the OS drag, so
    /// the originating surface must not keep treating later pointer motion as a
    /// continuation of the in-window press.
    pub(crate) fn cancel_pointer_capture(&mut self) {
        self.interaction.pointer.capture = None;
        self.interaction.pointer.capture_state = None;
        self.interaction.pointer.scroll_drag_capture = None;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PointerMoveDispatch {
    pub(super) target: Option<WidgetId>,
    pub(super) emitted_output: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::Vector2,
        layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams},
        runtime::{Event, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{PointerButton, PointerModifiers, TextInputWidget, TextWidget, WidgetSizing},
    };
    use std::sync::Arc;

    struct FocusTestBridge;

    impl RuntimeBridge<usize> for FocusTestBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            Arc::new(UiSurface::new(SurfaceNode::column(
                1,
                0.0,
                vec![
                    fixed_child(
                        28.0,
                        SurfaceNode::widget(
                            TextInputWidget::new(
                                10,
                                "tag",
                                WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    ),
                    fixed_child(
                        28.0,
                        SurfaceNode::widget(
                            TextWidget::new(
                                20,
                                "Passive hit target",
                                WidgetSizing::fixed(Vector2::new(160.0, 28.0)),
                            ),
                            WidgetMessageMapper::dynamic(|_| Some(20)),
                        ),
                    ),
                ],
            )))
        }

        fn reduce_message(&mut self, _message: usize) {}
    }

    fn fixed_child<Message>(height: f32, child: SurfaceNode<Message>) -> SurfaceChild<Message> {
        SurfaceChild::new(
            SlotParams {
                size_main: SizeModeMain::Fixed(height),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child,
        )
    }

    #[test]
    fn pointer_press_on_non_focusable_hit_target_clears_existing_focus() {
        let mut runtime = SurfaceRuntime::new(FocusTestBridge, Vector2::new(200.0, 80.0));

        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(4.0, 4.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        });
        assert_eq!(runtime.focused_widget(), Some(10));

        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(4.0, 32.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        });

        assert_eq!(runtime.focused_widget(), None);
    }
}
