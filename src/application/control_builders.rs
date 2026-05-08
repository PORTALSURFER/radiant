/// Builder for buttons that can emit messages or mutate state directly.
pub struct ButtonBuilder {
    label: String,
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
        let mut button = ButtonWidget::new(0, &self.label, default_button_sizing(&self.label));
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
        label: label.into(),
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

/// Builder for compact drag handles that can emit messages or mutate state directly.
pub struct DragHandleBuilder;

impl DragHandleBuilder {
    /// Emit a mapped host message for drag lifecycle events.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(crate::widgets::DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        view_node_from_widget(MappedWidget::new(
            DragHandleWidget::new(0, default_drag_handle_sizing()),
            WidgetMessageMapper::drag_handle(map),
        ))
    }

    /// Mutate application state directly when the handle is dragged.
    pub fn on_drag<State: 'static>(
        self,
        apply: impl Fn(&mut State, crate::widgets::DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.mapped(move |message| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, message))
        })
    }
}

/// Build a compact drag handle for pointer-driven reordering.
pub fn drag_handle() -> DragHandleBuilder {
    DragHandleBuilder
}

/// Build a drag handle with a custom widget-message mapper.
pub fn drag_handle_mapped<Message: 'static>(
    map: impl Fn(crate::widgets::DragHandleMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    drag_handle().mapped(map)
}

/// Builder for toggles that can emit messages or mutate state directly.
pub struct ToggleBuilder {
    label: String,
    checked: bool,
    compact: bool,
    style: Option<WidgetStyle>,
}

impl ToggleBuilder {
    /// Apply an explicit widget style before binding this toggle.
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

    /// Emit a host message mapped from checked state.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut node = view_node_from_widget(MappedWidget::new(
            ToggleWidget::new(
                0,
                &self.label,
                default_toggle_sizing(&self.label, self.compact),
            )
            .with_checked(self.checked),
            WidgetMessageMapper::toggle(move |message| match message {
                crate::widgets::ToggleMessage::ValueChanged { checked } => map(checked),
            }),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when checked state changes.
    pub fn on_change<State: 'static>(
        self,
        apply: impl Fn(&mut State, bool) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.message(move |checked| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, checked))
        })
    }
}

/// Build a toggle.
pub fn toggle(label: impl Into<String>, checked: bool) -> ToggleBuilder {
    ToggleBuilder {
        label: label.into(),
        checked,
        compact: false,
        style: None,
    }
}

/// Build a compact checkbox.
pub fn checkbox(checked: bool) -> ToggleBuilder {
    ToggleBuilder {
        label: String::new(),
        checked,
        compact: true,
        style: None,
    }
}

/// Build a toggle that maps value changes by checked state.
pub fn toggle_mapped<Message: 'static>(
    label: impl Into<String>,
    checked: bool,
    map: impl Fn(bool) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    toggle(label, checked).message(map)
}

/// Builder for text inputs that can emit messages or mutate state directly.
pub struct TextInputBuilder {
    value: String,
    placeholder: Option<String>,
    style: Option<WidgetStyle>,
}

impl TextInputBuilder {
    /// Show placeholder text when the input value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Apply an explicit widget style before binding this text input.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Emit a host message mapped from the input value.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut input = TextInputWidget::new(0, self.value, default_text_input_sizing());
        input.props.placeholder = self.placeholder;
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(move |message| match message {
                crate::widgets::TextInputMessage::Changed { value }
                | crate::widgets::TextInputMessage::Submitted { value } => map(value),
            }),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when the input value changes.
    pub fn on_change<State: 'static>(
        self,
        apply: impl Fn(&mut State, String) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.message(move |value| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, value.clone()))
        })
    }

    /// Bind this input to a mutable `String` field on application state.
    pub fn bind<State: 'static>(
        self,
        field: impl for<'a> Fn(&'a mut State) -> &'a mut String + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        self.on_change(move |state, value| *field(state) = value)
    }

    /// Bind edits to a mutable `String` field and run a state callback on submit.
    pub fn bind_submit<State: 'static>(
        self,
        field: impl for<'a> Fn(&'a mut State) -> &'a mut String + Send + Sync + 'static,
        submit: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let field = Arc::new(field);
        let submit = Arc::new(submit);
        let mut input = TextInputWidget::new(0, self.value, default_text_input_sizing());
        input.props.placeholder = self.placeholder;
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(move |message| {
                let field = Arc::clone(&field);
                let submit = Arc::clone(&submit);
                StateAction::new(move |state| match &message {
                    crate::widgets::TextInputMessage::Changed { value } => {
                        *field(state) = value.clone();
                    }
                    crate::widgets::TextInputMessage::Submitted { value } => {
                        *field(state) = value.clone();
                        submit(state);
                    }
                })
            }),
        ));
        node.style = self.style;
        node
    }
}

/// Build a single-line text input.
pub fn text_input(value: impl Into<String>) -> TextInputBuilder {
    TextInputBuilder {
        value: value.into(),
        placeholder: None,
        style: None,
    }
}

/// Build a single-line text input that maps edits and submissions by value.
pub fn text_input_mapped<Message: 'static>(
    value: impl Into<String>,
    map: impl Fn(String) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    text_input(value).message(map)
}

