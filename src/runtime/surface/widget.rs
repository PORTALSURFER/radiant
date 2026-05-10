use crate::{
    gui::types::Rect,
    layout::LayoutNode,
    widgets::{FocusBehavior, Widget, WidgetId, WidgetInput, WidgetOutput},
};
use std::sync::Arc;

/// Shared mapper type that turns widget-specific payloads into host-defined messages.
pub type MessageMapper<Input, Message> = Arc<dyn Fn(Input) -> Message + Send + Sync>;

/// Message bindings that turn widget output payloads into host-defined messages.
#[derive(Default)]
pub struct WidgetMessageMapper<Message> {
    map: Option<Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>>,
}

impl<Message> Clone for WidgetMessageMapper<Message> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.as_ref().map(Arc::clone),
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a mapper that does not emit host-defined messages.
    pub fn none() -> Self {
        Self { map: None }
    }

    /// Build a mapper for any typed widget output payload.
    pub fn typed<Output>(map: impl Fn(Output) -> Message + Send + Sync + 'static) -> Self
    where
        Output: Clone + Send + Sync + 'static,
    {
        Self::dynamic(move |output| output.typed_ref::<Output>().cloned().map(&map))
    }

    /// Build a dynamic output mapper for custom widgets.
    pub fn dynamic(map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static) -> Self {
        Self {
            map: Some(Arc::new(map)),
        }
    }

    pub(super) fn map_output(&self, output: WidgetOutput) -> Option<Message> {
        self.map.as_ref().and_then(|map| map(output))
    }
}

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
                || self.messages.map.is_some())
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
