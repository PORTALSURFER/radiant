use crate::{
    application::{MappedWidget, ViewNode, default_text_input_sizing, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{TextInputChrome, TextInputMessage, TextInputWidget, WidgetProminence, WidgetStyle},
};

/// Builder for text inputs that can emit messages or mutate state directly.
pub struct TextInputBuilder {
    value: String,
    placeholder: Option<String>,
    completion_suffix: Option<String>,
    style: Option<WidgetStyle>,
    selection: Option<(usize, usize)>,
    chrome: TextInputChrome,
}

impl TextInputBuilder {
    /// Show placeholder text when the input value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Show inline completion text after the current value.
    pub fn completion_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        if !suffix.is_empty() {
            self.completion_suffix = Some(suffix);
        }
        self
    }

    /// Apply an explicit widget style before binding this text input.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set the initial selection anchor and caret, measured in Unicode scalar values.
    pub fn selection(mut self, anchor: usize, caret: usize) -> Self {
        self.selection = Some((anchor, caret));
        self
    }

    /// Select the full input value when the widget is created.
    pub fn select_all(mut self) -> Self {
        let end = self.value.chars().count();
        self.selection = Some((0, end));
        self
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Use a minimal underline-only input chrome instead of a boxed field.
    pub fn underline(mut self) -> Self {
        self.chrome = TextInputChrome::Underline;
        self
    }

    /// Emit a host message mapped from the input value.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.message_event(move |message| map(message.into_value()))
    }

    /// Emit a host message mapped from the full text-input event.
    pub fn message_event<Message: 'static>(
        self,
        map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let (input, style) = self.into_widget_and_style();
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(map),
        ));
        node.style = style;
        node
    }

    fn into_widget_and_style(self) -> (TextInputWidget, Option<WidgetStyle>) {
        let Self {
            value,
            placeholder,
            completion_suffix,
            style,
            selection,
            chrome,
        } = self;
        let mut input = TextInputWidget::new(0, value, default_text_input_sizing());
        input.props.placeholder = placeholder.map(Into::into);
        input.props.completion_suffix = completion_suffix.map(Into::into);
        input.props.chrome = chrome;
        if let Some((anchor, caret)) = selection {
            input.state.selection_anchor = anchor;
            input.state.caret = caret;
        }
        (input, style)
    }
}

/// Build a single-line text input.
pub fn text_input(value: impl Into<String>) -> TextInputBuilder {
    TextInputBuilder {
        value: value.into(),
        placeholder: None,
        completion_suffix: None,
        style: None,
        selection: None,
        chrome: TextInputChrome::Full,
    }
}

/// Build a single-line text input that maps edits and submissions by value.
pub fn text_input_mapped<Message: 'static>(
    value: impl Into<String>,
    map: impl Fn(String) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    text_input(value).message(map)
}
