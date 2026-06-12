use crate::application::{Layer, ViewNode};

/// Declarative collection of view-local scene overlays.
///
/// Use this when a component owns several overlays and wants to keep
/// permanent layout composition separate from overlay projection.
pub struct Overlays<Message> {
    layers: Vec<Layer<Message>>,
}

/// Build an empty collection of typed overlays.
pub fn overlays<Message>() -> Overlays<Message> {
    Overlays { layers: Vec::new() }
}

impl<Message> Overlays<Message> {
    /// Add one typed overlay.
    pub fn layer(mut self, layer: Layer<Message>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Add one optional typed overlay.
    pub fn layer_opt(self, layer: Option<Layer<Message>>) -> Self {
        match layer {
            Some(layer) => self.layer(layer),
            None => self,
        }
    }

    /// Add typed overlays in declaration order.
    pub fn layers(mut self, layers: impl IntoIterator<Item = Layer<Message>>) -> Self {
        self.layers.extend(layers);
        self
    }

    /// Add one floating overlay.
    pub fn floating(self, view: ViewNode<Message>) -> Self {
        self.layer(Layer::floating(view))
    }

    /// Add one optional floating overlay.
    pub fn floating_opt(self, view: Option<ViewNode<Message>>) -> Self {
        match view {
            Some(view) => self.floating(view),
            None => self,
        }
    }

    /// Add one popover overlay.
    pub fn popover(self, view: ViewNode<Message>) -> Self {
        self.layer(Layer::popover(view))
    }

    /// Add one optional popover overlay.
    pub fn popover_opt(self, view: Option<ViewNode<Message>>) -> Self {
        match view {
            Some(view) => self.popover(view),
            None => self,
        }
    }

    /// Add one modal overlay.
    pub fn modal(self, view: ViewNode<Message>) -> Self {
        self.layer(Layer::modal(view))
    }

    /// Add one optional modal overlay.
    pub fn modal_opt(self, view: Option<ViewNode<Message>>) -> Self {
        match view {
            Some(view) => self.modal(view),
            None => self,
        }
    }

    /// Add one modal overlay that blocks input behind it.
    pub fn blocking_modal(self, view: ViewNode<Message>) -> Self
    where
        Message: 'static,
    {
        self.layer(Layer::modal(view).block_input())
    }

    /// Add one optional modal overlay that blocks input behind it.
    pub fn blocking_modal_opt(self, view: Option<ViewNode<Message>>) -> Self
    where
        Message: 'static,
    {
        match view {
            Some(view) => self.blocking_modal(view),
            None => self,
        }
    }

    /// Add one context-menu overlay.
    pub fn context_menu(self, view: ViewNode<Message>) -> Self {
        self.layer(Layer::context_menu(view))
    }

    /// Add one optional context-menu overlay.
    pub fn context_menu_opt(self, view: Option<ViewNode<Message>>) -> Self {
        match view {
            Some(view) => self.context_menu(view),
            None => self,
        }
    }

    /// Add one context-menu overlay that emits `message` for outside pointer
    /// activation.
    pub fn dismissible_context_menu(self, view: ViewNode<Message>, message: Message) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.layer(Layer::context_menu(view).dismiss_on_outside_click(message))
    }

    /// Add one optional context-menu overlay that emits `message` for outside
    /// pointer activation.
    pub fn dismissible_context_menu_opt(
        self,
        view: Option<ViewNode<Message>>,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        match view {
            Some(view) => self.dismissible_context_menu(view, message),
            None => self,
        }
    }

    /// Add one tooltip overlay.
    pub fn tooltip(self, view: ViewNode<Message>) -> Self {
        self.layer(Layer::tooltip(view))
    }

    /// Add one optional tooltip overlay.
    pub fn tooltip_opt(self, view: Option<ViewNode<Message>>) -> Self {
        match view {
            Some(view) => self.tooltip(view),
            None => self,
        }
    }

    /// Add one drag-preview overlay.
    pub fn drag_preview(self, view: ViewNode<Message>) -> Self {
        self.layer(Layer::drag_preview(view))
    }

    /// Add one optional drag-preview overlay.
    pub fn drag_preview_opt(self, view: Option<ViewNode<Message>>) -> Self {
        match view {
            Some(view) => self.drag_preview(view),
            None => self,
        }
    }

    pub(in crate::application) fn into_layers(self) -> Vec<Layer<Message>> {
        self.layers
    }
}
