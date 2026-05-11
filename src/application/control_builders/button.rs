use super::*;

/// Builder for buttons that can emit messages or mutate state directly.
pub struct ButtonBuilder {
    label: PaintText,
    style: Option<WidgetStyle>,
    secondary_click: bool,
    drag: bool,
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
        map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let sizing = default_button_sizing(&self.label);
        let mut button = ButtonWidget::new(0, self.label, sizing);
        if self.secondary_click {
            button = button.with_secondary_click();
        }
        if self.drag {
            button = button.with_drag();
        }
        let mut node =
            view_node_from_widget(MappedWidget::new(button, WidgetMessageMapper::button(map)));
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

    /// Mutate application state on primary or secondary/right activation.
    pub fn on_click_or_secondary<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        self.secondary_clicks().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { .. } => secondary(state),
                crate::widgets::ButtonMessage::Drag(_) => {}
            })
        })
    }

    /// Mutate application state on primary activation or secondary/right
    /// activation with pointer position.
    pub fn on_click_or_secondary_at<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State, crate::gui::types::Point) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        self.secondary_clicks().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { position } => {
                    secondary(state, position);
                }
                crate::widgets::ButtonMessage::Drag(_) => {}
            })
        })
    }

    /// Mutate application state on primary, secondary/right, or drag lifecycle messages.
    pub fn on_click_secondary_or_drag<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State) + Send + Sync + 'static,
        drag: impl Fn(&mut State, crate::widgets::DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        let drag = Arc::new(drag);
        self.secondary_clicks().draggable().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            let drag = Arc::clone(&drag);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { .. } => secondary(state),
                crate::widgets::ButtonMessage::Drag(message) => drag(state, message),
            })
        })
    }

    /// Mutate application state on primary, secondary/right with pointer
    /// position, or drag lifecycle messages.
    pub fn on_click_secondary_at_or_drag<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State, crate::gui::types::Point) + Send + Sync + 'static,
        drag: impl Fn(&mut State, crate::widgets::DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        let drag = Arc::new(drag);
        self.secondary_clicks().draggable().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            let drag = Arc::clone(&drag);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { position } => {
                    secondary(state, position);
                }
                crate::widgets::ButtonMessage::Drag(message) => drag(state, message),
            })
        })
    }
}

/// Build a button.
pub fn button(label: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder {
        label: PaintText::from(label.into()),
        style: None,
        secondary_click: false,
        drag: false,
    }
}

/// Build a button that emits one cloned host message when activated.
pub fn button_message<Message>(label: impl Into<String>, message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button(label).message(message)
}

/// Build a button with a custom widget-message mapper.
pub fn button_mapped<Message: 'static>(
    label: impl Into<String>,
    map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    button(label).mapped(map)
}
