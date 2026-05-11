use super::*;

/// Builder for badges that can emit messages or mutate state directly.
pub struct BadgeBuilder {
    label: PaintText,
    style: Option<WidgetStyle>,
}

impl BadgeBuilder {
    /// Apply an explicit widget style before binding this badge.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone.
    pub fn danger(self) -> Self {
        self.style(danger_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

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
        map: impl Fn(crate::widgets::BadgeMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let sizing = default_badge_sizing(&self.label);
        let mut node = view_node_from_widget(MappedWidget::new(
            BadgeWidget::new(0, self.label, sizing),
            WidgetMessageMapper::badge(map),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when activated.
    pub fn on_click<State: 'static>(
        self,
        apply: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        self.message(StateAction::new(apply))
    }
}

/// Build a badge or pill.
pub fn badge(label: impl Into<String>) -> BadgeBuilder {
    BadgeBuilder {
        label: PaintText::from(label.into()),
        style: None,
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
    map: impl Fn(crate::widgets::BadgeMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    badge(label).mapped(map)
}
