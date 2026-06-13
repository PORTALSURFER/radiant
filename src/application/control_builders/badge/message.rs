use crate::{
    application::{MappedWidget, ViewNode, default_badge_sizing, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{BadgeMessage, BadgeWidget},
};

use super::{BadgeBuilder, badge};

impl BadgeBuilder {
    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.mapped(move |_| message.clone())
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let sizing = default_badge_sizing(&self.label);
        let badge = BadgeWidget::new(0, self.label, sizing).with_active(self.active);
        let mut node =
            view_node_from_widget(MappedWidget::new(badge, WidgetMessageMapper::badge(map)));
        node.style = self.style;
        node
    }
}

/// Build a badge that emits one cloned host message when activated.
pub fn badge_message<Message>(label: impl Into<String>, message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    badge(label).message(message)
}

/// Build a badge with a custom widget-message mapper.
pub fn badge_mapped<Message: 'static>(
    label: impl Into<String>,
    map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    badge(label).mapped(map)
}
