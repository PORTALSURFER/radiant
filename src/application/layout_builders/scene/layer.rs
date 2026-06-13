use crate::{
    application::{Layer, LayerInputPolicy, ViewNode, pointer_shield},
    runtime::LayerKind,
    widgets::PointerShieldMessage,
};

impl<Message> Layer<Message> {
    /// Build a generic floating layer above base content.
    pub fn floating(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Floating, view)
    }

    /// Build a popover layer above generic floating layers.
    pub fn popover(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Popover, view)
    }

    /// Build a modal layer above popovers.
    pub fn modal(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Modal, view)
    }

    /// Build a context menu layer above modals.
    pub fn context_menu(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::ContextMenu, view)
    }

    /// Build a tooltip layer above context menus.
    pub fn tooltip(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::Tooltip, view)
    }

    /// Build a drag preview layer above every other transient category.
    pub fn drag_preview(view: ViewNode<Message>) -> Self {
        Self::new(LayerKind::DragPreview, view)
    }

    /// Keep this layer from adding any synthesized scene input surface.
    pub fn pass_through(mut self) -> Self {
        self.input_policy = LayerInputPolicy::PassThrough;
        self.input = None;
        self
    }

    /// Consume pointer and wheel input over the full scene behind this layer.
    pub fn block_input(mut self) -> Self
    where
        Message: 'static,
    {
        self.input_policy = LayerInputPolicy::BlockInput;
        self.input = Some(pointer_shield(true).consume());
        self
    }

    /// Emit a message on outside pointer activation and block wheel input
    /// behind this layer.
    pub fn dismiss_on_outside_click(mut self, message: Message) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.input_policy = LayerInputPolicy::DismissOnOutsideClick;
        self.input = Some(pointer_shield(true).filter_map(
            move |shield_message| match shield_message {
                PointerShieldMessage::PointerPress { .. } => Some(message.clone()),
                PointerShieldMessage::PointerMove { .. }
                | PointerShieldMessage::PointerRelease { .. }
                | PointerShieldMessage::PointerDrop { .. }
                | PointerShieldMessage::Wheel { .. } => None,
            },
        ));
        self
    }

    /// Return this layer's declared input behavior.
    pub const fn input_policy(&self) -> LayerInputPolicy {
        self.input_policy
    }
}
