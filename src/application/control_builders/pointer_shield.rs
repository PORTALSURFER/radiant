use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    gui::types::Point,
    runtime::WidgetMessageMapper,
    widgets::{PointerShieldMessage, PointerShieldWidget},
};

/// Builder for transparent pointer interception layers.
pub struct PointerShieldBuilder {
    widget: PointerShieldWidget,
}

impl PointerShieldBuilder {
    /// Configure whether the shield currently intercepts pointer input.
    pub fn active(mut self, active: bool) -> Self {
        self.widget = self.widget.active(active);
        self
    }

    /// Configure whether pointer movement is intercepted.
    pub fn pointer_move(mut self, enabled: bool) -> Self {
        self.widget = self.widget.with_pointer_move(enabled);
        self
    }

    /// Configure whether pointer press events are intercepted.
    pub fn pointer_press(mut self, enabled: bool) -> Self {
        self.widget = self.widget.with_pointer_press(enabled);
        self
    }

    /// Configure whether pointer release events are intercepted.
    pub fn pointer_release(mut self, enabled: bool) -> Self {
        self.widget = self.widget.with_pointer_release(enabled);
        self
    }

    /// Configure whether captured pointer drops are intercepted.
    pub fn pointer_drop(mut self, enabled: bool) -> Self {
        self.widget = self.widget.with_pointer_drop(enabled);
        self
    }

    /// Configure whether wheel input is intercepted.
    pub fn wheel(mut self, enabled: bool) -> Self {
        self.widget = self.widget.with_wheel(enabled);
        self
    }

    /// Emit a mapped host message when the pointer shield emits output.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(PointerShieldMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        view_node_from_widget(MappedWidget::new(
            self.widget,
            WidgetMessageMapper::typed(map),
        ))
    }

    /// Emit host messages for selected pointer shield outputs.
    pub fn filter_map<Message: 'static>(
        self,
        map: impl Fn(PointerShieldMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        view_node_from_widget(MappedWidget::new(
            self.widget,
            WidgetMessageMapper::dynamic(move |output| {
                output
                    .typed_ref::<PointerShieldMessage>()
                    .and_then(|message| map(*message))
            }),
        ))
    }

    /// Consume selected pointer and wheel input without emitting host messages.
    pub fn consume<Message: 'static>(self) -> ViewNode<Message> {
        self.filter_map(|_| None)
    }

    /// Build this shield as a passive input layer that emits no host messages.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(self.widget)
    }

    /// Emit a cloned host message only when the shield receives a pointer drop.
    pub fn on_drop<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        view_node_from_widget(MappedWidget::new(
            self.widget,
            WidgetMessageMapper::dynamic(move |output| {
                output
                    .typed_ref::<PointerShieldMessage>()
                    .and_then(|message_payload| match message_payload {
                        PointerShieldMessage::PointerDrop { .. } => Some(message.clone()),
                        _ => None,
                    })
            }),
        ))
    }

    /// Emit a host message from pointer movement positions only.
    pub fn on_pointer_move<Message: 'static>(
        self,
        map: impl Fn(Point) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        view_node_from_widget(MappedWidget::new(
            self.widget,
            WidgetMessageMapper::dynamic(move |output| {
                output
                    .typed_ref::<PointerShieldMessage>()
                    .and_then(|message| match message {
                        PointerShieldMessage::PointerMove { position } => Some(map(*position)),
                        _ => None,
                    })
            }),
        ))
    }
}

/// Build a transparent pointer shield with explicit event policy.
pub fn pointer_shield(active: bool) -> PointerShieldBuilder {
    PointerShieldBuilder {
        widget: PointerShieldWidget::fill(active),
    }
}

/// Build a pointer shield that only reports pointer movement.
pub fn pointer_move_shield(active: bool) -> PointerShieldBuilder {
    PointerShieldBuilder {
        widget: PointerShieldWidget::pointer_move_only(active),
    }
}

/// Build a pointer shield that only reports captured pointer drops.
pub fn pointer_drop_shield(active: bool) -> PointerShieldBuilder {
    PointerShieldBuilder {
        widget: PointerShieldWidget::pointer_drop_only(active),
    }
}
