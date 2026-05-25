use super::{PointerMoveDispatch, SurfaceRuntime};
use crate::{
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn dispatch_pointer_move_target(
        &mut self,
        position: Point,
    ) -> PointerMoveDispatch {
        let mut emitted_output = false;
        self.update_drag_preview_position(position);
        if self.drag_scrollbar_to(position) {
            return PointerMoveDispatch::default();
        }
        self.update_hovered_scroll_affordance(position);

        let pointer_widget = self.pointer_widget_for_move(position);
        self.update_hovered_container(position, pointer_widget);

        let hover_widget = self.hover_widget_for_move(position, pointer_widget);
        let hover_changed = self.route_hover_transition(position, hover_widget);
        emitted_output |= hover_changed.emitted_output;
        emitted_output |= self.route_captured_pass_through_move(position, pointer_widget);

        let Some(target) = self.interaction.pointer.capture.or(pointer_widget) else {
            return PointerMoveDispatch {
                target: None,
                emitted_output,
            };
        };
        self.route_pointer_move_to_target(position, target, hover_changed, emitted_output)
    }

    fn route_pointer_move_to_target(
        &mut self,
        position: Point,
        target: WidgetId,
        hover_changed: PointerHoverTransition,
        mut emitted_output: bool,
    ) -> PointerMoveDispatch {
        let accepts_stable_pointer_move = self.widget_accepts_stable_pointer_move(target);
        if !hover_changed.changed
            && self.interaction.pointer.capture.is_none()
            && !accepts_stable_pointer_move
        {
            return PointerMoveDispatch {
                target: Some(target),
                emitted_output,
            };
        }
        let routed = self.dispatch_input_output(target, WidgetInput::PointerMove { position });
        if let Some(emitted) = routed {
            // Stable pointer-move widgets may update local paint-only hover
            // state without emitting host messages. Captured drags can also
            // update local preview state even when the widget opts out of
            // stable hover motion. Request repaint here so cursor, handle, and
            // drag previews stay responsive without reducer churn.
            if accepts_stable_pointer_move || self.interaction.pointer.capture.is_some() {
                self.repaint_requested = true;
            }
            emitted_output |= emitted;
        }
        PointerMoveDispatch {
            target: routed.map(|_| target),
            emitted_output,
        }
    }

    fn update_drag_preview_position(&mut self, position: Point) {
        let Some(session) = self.interaction.drag.session.as_mut() else {
            return;
        };
        if session.pointer == position && session.visible {
            return;
        }
        session.pointer = position;
        session.visible = true;
        self.repaint_requested = true;
    }

    fn update_hovered_scroll_affordance(&mut self, position: Point) {
        let hovered_scroll_affordance = self.scroll_affordance_at(position);
        if self.interaction.hover.scroll_affordance == hovered_scroll_affordance {
            return;
        }
        self.interaction.hover.scroll_affordance = hovered_scroll_affordance;
        self.repaint_requested = true;
    }

    fn pointer_widget_for_move(&self, position: Point) -> Option<WidgetId> {
        if self.interaction.pointer.capture.is_some() {
            self.widget_at(position)
        } else {
            self.pointer_widget_at_for_move(position)
        }
    }

    fn update_hovered_container(&mut self, position: Point, pointer_widget: Option<WidgetId>) {
        let hover_container = if self.widget_suppresses_container_hover(pointer_widget) {
            None
        } else {
            self.styled_container_at(position)
        };
        if self.interaction.hover.container == hover_container {
            return;
        }
        self.interaction.hover.container = hover_container;
        self.repaint_requested = true;
    }

    fn hover_widget_for_move(
        &self,
        position: Point,
        pointer_widget: Option<WidgetId>,
    ) -> Option<WidgetId> {
        self.interaction
            .pointer
            .capture
            .filter(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(position))
            })
            .or_else(|| {
                self.interaction
                    .pointer
                    .capture
                    .is_none()
                    .then_some(pointer_widget)
                    .flatten()
            })
    }

    fn route_hover_transition(
        &mut self,
        position: Point,
        hover_widget: Option<WidgetId>,
    ) -> PointerHoverTransition {
        if self.interaction.hover.widget == hover_widget {
            return PointerHoverTransition {
                changed: false,
                emitted_output: false,
            };
        }
        let emitted_output = self
            .interaction
            .hover
            .widget
            .and_then(|previous| {
                self.dispatch_input_output(previous, WidgetInput::PointerMove { position })
            })
            .unwrap_or(false);
        self.interaction.hover.widget = hover_widget;
        PointerHoverTransition {
            changed: true,
            emitted_output,
        }
    }

    fn route_captured_pass_through_move(
        &mut self,
        position: Point,
        pointer_widget: Option<WidgetId>,
    ) -> bool {
        let (Some(captured), Some(pointer_widget)) =
            (self.interaction.pointer.capture, pointer_widget)
        else {
            return false;
        };
        if pointer_widget == captured || !self.widget_accepts_stable_pointer_move(pointer_widget) {
            return false;
        }
        let emitted =
            self.dispatch_input_output(pointer_widget, WidgetInput::PointerMove { position });
        if emitted.is_some() {
            self.repaint_requested = true;
        }
        emitted.unwrap_or(false)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PointerHoverTransition {
    changed: bool,
    emitted_output: bool,
}
