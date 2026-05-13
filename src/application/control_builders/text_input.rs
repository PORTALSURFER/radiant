use crate::{
    application::{
        MappedWidget, StateAction, ViewNode, default_text_input_sizing, view_node_from_widget,
    },
    runtime::WidgetMessageMapper,
    widgets::{TextInputMessage, TextInputWidget, WidgetProminence, WidgetStyle},
};
use std::sync::Arc;

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
        input.props.placeholder = self.placeholder.map(Into::into);
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(move |message| match message {
                TextInputMessage::Changed { value } | TextInputMessage::Submitted { value } => {
                    map(value)
                }
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
        input.props.placeholder = self.placeholder.map(Into::into);
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(move |message| {
                let field = Arc::clone(&field);
                let submit = Arc::clone(&submit);
                StateAction::new(move |state| match &message {
                    TextInputMessage::Changed { value } => {
                        *field(state) = value.clone();
                    }
                    TextInputMessage::Submitted { value } => {
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
