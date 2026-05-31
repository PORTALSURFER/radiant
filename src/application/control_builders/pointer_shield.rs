use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
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
