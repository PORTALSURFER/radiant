use std::sync::Arc;

use crate::{
    application::{
        MappedWidget, ViewNode, close_button, default_text_input_sizing, fixed_slot_if, row,
        view_node_from_widget,
    },
    runtime::WidgetMessageMapper,
    widgets::{
        TextInputChrome, TextInputMessage, TextInputWidget, WidgetId, WidgetProminence,
        WidgetStyle, stable_widget_id,
    },
};

const DEFAULT_TEXT_INPUT_CLEAR_BUTTON_HEIGHT: f32 = 20.0;
const DEFAULT_TEXT_INPUT_CLEAR_BUTTON_SIZE: f32 = 20.0;
const DEFAULT_TEXT_INPUT_CLEAR_BUTTON_SPACING: f32 = 4.0;
const TEXT_INPUT_CLEAR_BUTTON_KEY: &str = "clear";

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

    /// Add a reserved trailing clear-button slot that emits one cloned host message.
    pub fn clear_button<Message>(self, message: Message) -> TextInputWithClearButtonBuilder<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.clear_button_mapped(move || message.clone())
    }

    /// Add a reserved trailing clear-button slot that emits a mapped host message.
    pub fn clear_button_mapped<Message: 'static>(
        self,
        map: impl Fn() -> Message + Send + Sync + 'static,
    ) -> TextInputWithClearButtonBuilder<Message> {
        let clear_visible = !self.value.is_empty();
        TextInputWithClearButtonBuilder {
            input: self,
            clear_message: Arc::new(map),
            clear_visible,
            input_id: None,
            clear_button_id: None,
            input_key: None,
            clear_button_key: None,
            height: DEFAULT_TEXT_INPUT_CLEAR_BUTTON_HEIGHT,
            clear_button_size: DEFAULT_TEXT_INPUT_CLEAR_BUTTON_SIZE,
            spacing: DEFAULT_TEXT_INPUT_CLEAR_BUTTON_SPACING,
        }
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

/// Builder for a text input with a reserved optional clear-button slot.
///
/// The host still owns the value and message routing. Radiant owns the common
/// input row composition, stable clear-button slot, spacing, and hidden-button
/// behavior.
pub struct TextInputWithClearButtonBuilder<Message> {
    input: TextInputBuilder,
    clear_message: Arc<dyn Fn() -> Message + Send + Sync>,
    clear_visible: bool,
    input_id: Option<WidgetId>,
    clear_button_id: Option<WidgetId>,
    input_key: Option<String>,
    clear_button_key: Option<String>,
    height: f32,
    clear_button_size: f32,
    spacing: f32,
}

impl<Message> TextInputWithClearButtonBuilder<Message> {
    /// Set a stable widget id for the text input.
    ///
    /// The clear button receives a deterministic child id derived from this id
    /// unless `clear_button_id(...)` overrides it.
    pub fn id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self.clear_button_id = Some(text_input_clear_button_id(id));
        self.input_key = None;
        self.clear_button_key = None;
        self
    }

    /// Set a stable key for the text input.
    ///
    /// The clear button receives a deterministic child key derived from this
    /// key unless `clear_button_key(...)` overrides it.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.clear_button_key = Some(format!("{key}.{TEXT_INPUT_CLEAR_BUTTON_KEY}"));
        self.input_key = Some(key);
        self.input_id = None;
        self.clear_button_id = None;
        self
    }

    /// Override the derived clear-button widget id.
    ///
    /// Most app code should use `id(...)` and let Radiant derive this child id.
    /// This escape hatch is for focused tests, automation, accessibility, or
    /// host integrations that need an externally reserved child id.
    pub fn clear_button_id(mut self, id: WidgetId) -> Self {
        self.clear_button_id = Some(id);
        self.clear_button_key = None;
        self
    }

    /// Override the derived clear-button key.
    ///
    /// Most app code should use `key(...)` and let Radiant derive this child
    /// key.
    pub fn clear_button_key(mut self, key: impl Into<String>) -> Self {
        self.clear_button_key = Some(key.into());
        self.clear_button_id = None;
        self
    }

    /// Override fixed row and input height.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Override fixed clear-button slot size.
    pub const fn clear_button_size(mut self, size: f32) -> Self {
        self.clear_button_size = size;
        self
    }

    /// Override horizontal spacing between the input and clear-button slot.
    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<Message: 'static> TextInputWithClearButtonBuilder<Message> {
    /// Emit a host message mapped from the input value.
    pub fn message(
        self,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.message_event(move |message| map(message.into_value()))
    }

    /// Emit a host message mapped from the full text-input event.
    pub fn message_event(
        self,
        map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self {
            input,
            clear_message,
            clear_visible,
            input_id,
            clear_button_id,
            input_key,
            clear_button_key,
            height,
            clear_button_size,
            spacing,
        } = self;

        let mut input = input.message_event(map).fill_width().height(height);
        if let Some(id) = input_id {
            input = input.id(id);
        }
        if let Some(key) = input_key {
            input = input.key(key);
        }

        let clear = fixed_slot_if(
            clear_visible,
            || {
                let clear_message = Arc::clone(&clear_message);
                let mut clear = close_button().subtle().mapped(move |_| clear_message());
                if let Some(id) = clear_button_id {
                    clear = clear.id(id);
                }
                if let Some(key) = clear_button_key {
                    clear = clear.key(key);
                }
                clear
            },
            clear_button_size,
            clear_button_size,
        );

        row([input, clear])
            .spacing(spacing)
            .fill_width()
            .height(height)
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

/// Derive the default clear-button id for a text input with an explicit id.
///
/// Use this in tests, automation, or host integrations that need to address the
/// clear affordance generated by `text_input(...).clear_button(...)`.
pub fn text_input_clear_button_id(input_id: WidgetId) -> WidgetId {
    stable_widget_id(input_id, TEXT_INPUT_CLEAR_BUTTON_KEY)
}

#[cfg(test)]
mod tests {
    use super::text_input_clear_button_id;
    use crate::{
        application::{IntoView, column, text_input},
        layout::Vector2,
        widgets::{ButtonMessage, TextInputMessage, WidgetOutput},
    };

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Message {
        Input(String),
        Clear,
    }

    #[test]
    fn text_input_clear_button_applies_compact_default_geometry() {
        let layout = column([text_input("kick")
            .clear_button(Message::Clear)
            .id(10)
            .message(Message::Input)
            .id(1)])
        .view_layout_at_size(Vector2::new(160.0, 30.0));
        let clear_button_id = text_input_clear_button_id(10);

        assert_eq!(
            layout.rects[&1].height(),
            super::DEFAULT_TEXT_INPUT_CLEAR_BUTTON_HEIGHT
        );
        assert_eq!(
            layout.rects[&clear_button_id].width(),
            super::DEFAULT_TEXT_INPUT_CLEAR_BUTTON_SIZE
        );
        assert_eq!(
            layout.rects[&clear_button_id].min.x - layout.rects[&10].max.x,
            super::DEFAULT_TEXT_INPUT_CLEAR_BUTTON_SPACING
        );
    }

    #[test]
    fn text_input_clear_button_routes_input_and_clear_activation() {
        let surface = text_input("kick")
            .clear_button(Message::Clear)
            .id(10)
            .message(Message::Input)
            .into_surface();
        let clear_button_id = text_input_clear_button_id(10);

        assert_eq!(
            surface.dispatch_widget_output(
                10,
                WidgetOutput::typed(TextInputMessage::Changed {
                    value: String::from("snare"),
                }),
            ),
            Some(Message::Input(String::from("snare")))
        );
        assert_eq!(
            surface.dispatch_widget_output(
                clear_button_id,
                WidgetOutput::typed(ButtonMessage::Activate)
            ),
            Some(Message::Clear)
        );
    }

    #[test]
    fn text_input_clear_button_hides_inactive_button_but_reserves_slot() {
        let layout = text_input("")
            .clear_button(Message::Clear)
            .id(10)
            .message(Message::Input)
            .view_layout_at_size(Vector2::new(160.0, 30.0));
        let surface = text_input("")
            .clear_button(Message::Clear)
            .id(10)
            .message(Message::Input)
            .into_surface();
        let clear_button_id = text_input_clear_button_id(10);

        assert!(surface.find_widget(clear_button_id).is_none());
        assert!(!layout.rects.contains_key(&clear_button_id));
        assert_eq!(
            surface.dispatch_widget_output(
                clear_button_id,
                WidgetOutput::typed(ButtonMessage::Activate)
            ),
            None
        );
    }
}
