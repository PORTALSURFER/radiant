use crate::{
    application::{MappedWidget, ViewNode, primary_style, view_node_from_widget},
    gui::svg::SvgIcon,
    runtime::WidgetMessageMapper,
    widgets::{ButtonMessage, IconButtonWidget, WidgetProminence, WidgetSizing, WidgetStyle},
};

/// Builder for compact SVG icon buttons.
pub struct IconButtonBuilder {
    icon: SvgIcon,
    style: Option<WidgetStyle>,
    enabled: bool,
    active: bool,
}

impl IconButtonBuilder {
    /// Apply an explicit widget style before binding this icon button.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Set whether this button can be activated.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set whether this button should paint as active.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
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
        map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut widget = IconButtonWidget::new(
            0,
            self.icon,
            WidgetSizing::fixed(crate::layout::Vector2::new(28.0, 24.0)),
        );
        widget.common.state.disabled = !self.enabled;
        widget.common.state.active = self.active;
        let mut node = view_node_from_widget(MappedWidget::new(
            widget,
            WidgetMessageMapper::icon_button(map),
        ));
        node.style = self.style;
        node
    }
}

/// Build a compact SVG icon button.
pub fn icon_button(icon: SvgIcon) -> IconButtonBuilder {
    IconButtonBuilder {
        icon,
        style: None,
        enabled: true,
        active: false,
    }
}
