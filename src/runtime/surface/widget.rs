use crate::{
    gui::types::{Point, Rect},
    layout::LayoutNode,
    widgets::{FocusBehavior, Widget, WidgetCursor, WidgetId, WidgetInput, WidgetOutput},
};

mod mapper;

pub use mapper::{MessageMapper, ScrollMessageMapper, WidgetMessageMapper};

/// One widget leaf inside a generic declarative [`UiSurface`](super::UiSurface).
pub struct SurfaceWidget<Message> {
    widget: Box<dyn Widget>,
    messages: WidgetMessageMapper<Message>,
}

impl<Message> Clone for SurfaceWidget<Message> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            messages: self.messages.clone(),
        }
    }
}

impl<Message> SurfaceWidget<Message> {
    /// Build a widget leaf plus host-defined message mapper.
    pub fn new(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self {
            widget: Box::new(widget),
            messages,
        }
    }

    /// Build a custom widget leaf plus host-defined message mapper.
    pub fn custom(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self {
            widget: Box::new(widget),
            messages,
        }
    }

    /// Build a custom boxed widget leaf plus host-defined message mapper.
    pub fn custom_box(widget: Box<dyn Widget>, messages: WidgetMessageMapper<Message>) -> Self {
        Self { widget, messages }
    }

    /// Return the stable widget identifier.
    pub fn id(&self) -> WidgetId {
        self.widget.common().id
    }

    /// Return the runtime widget object.
    pub fn widget(&self) -> &dyn Widget {
        self.widget.as_ref()
    }

    /// Return the runtime widget object mutably.
    pub fn widget_mut(&mut self) -> &mut dyn Widget {
        self.widget.as_mut()
    }

    /// Return the runtime widget object.
    pub fn widget_object(&self) -> &dyn Widget {
        self.widget.as_ref()
    }

    /// Return the runtime widget object mutably.
    pub fn widget_object_mut(&mut self) -> &mut dyn Widget {
        self.widget.as_mut()
    }

    /// Return whether this widget participates in runtime focus management.
    pub fn is_focusable(&self) -> bool {
        self.widget.common().focus != FocusBehavior::None && !self.widget.common().state.disabled
    }

    /// Return whether this widget participates in keyboard focus traversal.
    pub fn is_keyboard_focusable(&self) -> bool {
        self.widget.common().focus == FocusBehavior::Keyboard
            && !self.widget.common().state.disabled
    }

    /// Return whether this widget can be a pointer hit-test target.
    pub fn receives_pointer_hit_testing(&self) -> bool {
        let common = self.widget.common();
        !common.state.disabled
            && (common.focus != FocusBehavior::None
                || common.paint.suppresses_container_hover
                || self.messages.maps_any_output())
    }

    pub(in crate::runtime) fn receives_wheel_input(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_wheel_input()
    }

    pub(in crate::runtime) fn accepts_pointer_move(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.accepts_pointer_move()
    }

    pub(in crate::runtime) fn prefers_pointer_move_paint_only(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.prefers_pointer_move_paint_only()
    }

    pub(in crate::runtime) fn allows_captured_pointer_pass_through(&self) -> bool {
        !self.widget.common().state.disabled && self.widget.allows_captured_pointer_pass_through()
    }

    pub(in crate::runtime) fn cursor_for_point(
        &self,
        bounds: Rect,
        point: Point,
    ) -> Option<WidgetCursor> {
        (!self.widget.common().state.disabled)
            .then(|| self.widget.cursor_for_point(bounds, point))
            .flatten()
    }

    pub(in crate::runtime) fn needs_state_synchronization(&self) -> bool {
        self.widget.needs_state_synchronization()
    }

    pub(in crate::runtime) fn suppresses_container_hover(&self) -> bool {
        let common = self.widget.common();
        !common.state.disabled
            && common.paint.paints_state_layers
            && (common.focus != FocusBehavior::None || common.paint.suppresses_container_hover)
    }

    pub(super) fn layout_node(&self) -> LayoutNode {
        self.widget.common().layout_node()
    }

    pub(super) fn handle_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        (self.id() == widget_id)
            .then(|| self.widget.handle_input(bounds, input))
            .flatten()
    }

    pub(in crate::runtime) fn dispatch_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> super::WidgetDispatchResult<Message> {
        let Some(output) = self.handle_input(widget_id, bounds, input) else {
            return super::WidgetDispatchResult::NoOutput;
        };
        self.messages
            .map_output(output)
            .map(super::WidgetDispatchResult::Message)
            .unwrap_or(super::WidgetDispatchResult::UnmappedOutput)
    }

    pub(super) fn dispatch_output(
        &self,
        widget_id: WidgetId,
        output: WidgetOutput,
    ) -> Option<Message> {
        (self.id() == widget_id)
            .then(|| self.messages.map_output(output))
            .flatten()
    }
}
