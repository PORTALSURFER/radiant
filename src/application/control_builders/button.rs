use crate::{
    application::{
        MappedWidget, TextContent, ViewNode, danger_style, default_button_sizing, primary_style,
        view_node_from_widget,
    },
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{
        ButtonMessage, ButtonWidget, DragHandleMessage, WidgetOutput, WidgetProminence, WidgetStyle,
    },
};

/// Builder for buttons that emit explicit host messages.
pub struct ButtonBuilder {
    label: PaintText,
    trailing_label: Option<PaintText>,
    style: Option<WidgetStyle>,
    secondary_click: bool,
    drag: bool,
    hover_chrome_only: bool,
}

impl ButtonBuilder {
    /// Apply an explicit widget style before binding this button.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone for destructive actions.
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

    /// Emit secondary/right-click messages from this button.
    pub fn secondary_clicks(mut self) -> Self {
        self.secondary_click = true;
        self
    }

    /// Emit drag lifecycle messages from the button surface.
    pub fn draggable(mut self) -> Self {
        self.drag = true;
        self
    }

    /// Paint button chrome only while the control is hovered, pressed, or focused.
    pub fn hover_chrome_only(mut self) -> Self {
        self.hover_chrome_only = true;
        self
    }

    pub(in crate::application) fn trailing_label(mut self, label: impl Into<TextContent>) -> Self {
        self.trailing_label = Some(label.into().into_paint_text());
        self
    }

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.with_message_mapper(WidgetMessageMapper::button_message(message))
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.with_message_mapper(WidgetMessageMapper::button(map))
    }

    /// Emit a host message for selected button outputs.
    pub fn filter_mapped<Message: 'static>(
        self,
        map: impl Fn(ButtonMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.with_message_mapper(WidgetMessageMapper::dynamic(move |output: WidgetOutput| {
            output.typed_ref::<ButtonMessage>().cloned().and_then(&map)
        }))
    }

    /// Emit one message when activated and mapped messages for drag lifecycle events.
    pub fn click_or_drag<Message>(
        self,
        activate: Message,
        drag: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.draggable()
            .filter_mapped(move |message| match message {
                ButtonMessage::Activate | ButtonMessage::ActivateWithModifiers { .. } => {
                    Some(activate.clone())
                }
                ButtonMessage::Drag(message) => Some(drag(message)),
                ButtonMessage::SecondaryActivate { .. } => None,
            })
    }

    fn with_message_mapper<Message: 'static>(
        self,
        messages: WidgetMessageMapper<Message>,
    ) -> ViewNode<Message> {
        let sizing = default_button_sizing(&self.label);
        let mut button = ButtonWidget::new(0, self.label, sizing);
        if let Some(trailing_label) = self.trailing_label {
            button = button.with_trailing_label(trailing_label);
        }
        if self.secondary_click {
            button = button.with_secondary_click();
        }
        if self.drag {
            button = button.with_drag();
        }
        if self.hover_chrome_only {
            button = button.with_hover_chrome_only();
        }
        let mut node = view_node_from_widget(MappedWidget::new(button, messages));
        node.style = self.style;
        node
    }
}

/// Build a button.
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder {
        label: label.into().into_paint_text(),
        trailing_label: None,
        style: None,
        secondary_click: false,
        drag: false,
        hover_chrome_only: false,
    }
}

/// Build a button that emits one cloned host message when activated.
pub fn button_message<Message>(label: impl Into<TextContent>, message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button(label).message(message)
}

/// Build a button with a custom widget-message mapper.
pub fn button_mapped<Message: 'static>(
    label: impl Into<TextContent>,
    map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    button(label).mapped(map)
}
