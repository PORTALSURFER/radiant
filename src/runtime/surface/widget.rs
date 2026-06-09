use crate::{
    gui::types::{Point, Rect},
    layout::LayoutNode,
    widgets::{
        FocusBehavior, PointerCapturePolicy, Widget, WidgetCursor, WidgetId, WidgetInput,
        WidgetOutput,
    },
};

mod mapper;

pub use mapper::{
    MessageMapper, NativeFileDropMessageMapper, ScrollMessageMapper, WidgetMessageMapper,
};

/// One widget leaf inside a generic declarative [`UiSurface`](super::UiSurface).
pub struct SurfaceWidget<Message> {
    widget: Box<dyn Widget>,
    messages: WidgetMessageMapper<Message>,
    accepts_native_file_drop: bool,
}

impl<Message> Clone for SurfaceWidget<Message> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            messages: self.messages.clone(),
            accepts_native_file_drop: self.accepts_native_file_drop,
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
            accepts_native_file_drop: false,
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
            accepts_native_file_drop: false,
        }
    }

    /// Build a custom boxed widget leaf plus host-defined message mapper.
    pub fn custom_box(widget: Box<dyn Widget>, messages: WidgetMessageMapper<Message>) -> Self {
        Self {
            widget,
            messages,
            accepts_native_file_drop: false,
        }
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

    pub(in crate::runtime) fn accepts_native_file_drop(&self) -> bool {
        !self.widget.common().state.disabled && self.accepts_native_file_drop
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

    pub(in crate::runtime) fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        if self.widget.common().state.disabled {
            PointerCapturePolicy::Exclusive
        } else {
            self.widget.pointer_capture_policy()
        }
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

    pub(in crate::runtime) fn dispatch_native_file_drop(
        &self,
        widget_id: WidgetId,
        drop: crate::runtime::NativeFileDrop,
    ) -> Option<Message> {
        (self.id() == widget_id)
            .then(|| self.messages.map_native_file_drop(drop))
            .flatten()
    }

    pub(in crate::runtime) fn with_native_file_drop(
        mut self,
        map: impl Fn(crate::runtime::NativeFileDrop) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.accepts_native_file_drop = true;
        self.messages = self.messages.with_native_file_drop(map);
        self
    }

    pub(in crate::runtime) fn accepting_native_file_drop(mut self) -> Self {
        self.accepts_native_file_drop = true;
        self
    }
}
