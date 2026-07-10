use crate::{
    application::{MappedWidget, TextContent, ViewNode, stack, text, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{
        InteractiveRowMessage, InteractiveRowWidget, TextAlign, WidgetProminence, WidgetSizing,
        WidgetStyle,
    },
};

/// Default action-row height in logical pixels.
pub const DEFAULT_ACTION_ROW_HEIGHT: f32 = 28.0;

/// Builder for compact clickable action rows.
///
/// Action rows are useful in context menus, command lists, and other dense
/// command surfaces where the interaction surface and label should compose
/// predictably without an application-owned custom widget.
pub struct ActionRowBuilder {
    label: TextContent,
    style: Option<WidgetStyle>,
}

impl ActionRowBuilder {
    /// Apply an explicit widget style before binding this action row.
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

    /// Emit one cloned host message when the row is activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        let label = self.label.clone();
        stack([
            action_row_hit_surface(
                self.style,
                WidgetMessageMapper::constant(message, |output| {
                    matches!(
                        output.typed_ref::<InteractiveRowMessage>(),
                        Some(InteractiveRowMessage::Activate)
                    )
                }),
            ),
            text(label)
                .align_text(TextAlign::Left)
                .truncate()
                .padding_x(8.0)
                .fill_width()
                .height(DEFAULT_ACTION_ROW_HEIGHT),
        ])
        .fill_width()
        .height(DEFAULT_ACTION_ROW_HEIGHT)
    }

    /// Emit a mapped host message when the row is activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let label = self.label.clone();
        stack([
            action_row_hit_surface(
                self.style,
                WidgetMessageMapper::dynamic(move |output| {
                    let message = output.typed_ref::<InteractiveRowMessage>()?;
                    matches!(message, InteractiveRowMessage::Activate).then(|| map(*message))
                }),
            ),
            text(label)
                .align_text(TextAlign::Left)
                .truncate()
                .padding_x(8.0)
                .fill_width()
                .height(DEFAULT_ACTION_ROW_HEIGHT),
        ])
        .fill_width()
        .height(DEFAULT_ACTION_ROW_HEIGHT)
    }
}

/// Build a compact clickable action row.
pub fn action_row(label: impl Into<TextContent>) -> ActionRowBuilder {
    ActionRowBuilder {
        label: label.into(),
        style: None,
    }
}

fn action_row_hit_surface<Message: 'static>(
    style: Option<WidgetStyle>,
    messages: WidgetMessageMapper<Message>,
) -> ViewNode<Message> {
    let mut node = view_node_from_widget(MappedWidget::new(
        InteractiveRowWidget::new(
            0,
            WidgetSizing::fixed(crate::layout::Vector2::new(1.0, DEFAULT_ACTION_ROW_HEIGHT)),
        ),
        messages,
    ));
    node.style = style;
    node.fill_width().height(DEFAULT_ACTION_ROW_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{app, column},
        gui::types::Point,
        layout::Vector2,
        runtime::SurfaceRuntime,
        widgets::{PointerButton, PointerModifiers, TextWidget, WidgetInput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum DemoMessage {
        Activate,
    }

    #[derive(Default)]
    struct DemoState {
        activated: bool,
    }

    #[test]
    fn action_row_emits_message_on_primary_activation() {
        let bridge = app(DemoState::default())
            .view(|state| {
                column([
                    action_row("Copy Path")
                        .message(DemoMessage::Activate)
                        .key("copy-action"),
                    text(if state.activated { "activated" } else { "idle" }).id(90),
                ])
            })
            .update(|state, message| match message {
                DemoMessage::Activate => state.activated = true,
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 52.0));
        let position = Point::new(12.0, 12.0);

        assert_eq!(feedback_text(&runtime), Some("idle"));
        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                modifiers: PointerModifiers::default(),
            },
        );
        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Secondary,
                modifiers: PointerModifiers::default(),
            },
        );
        assert_eq!(feedback_text(&runtime), Some("idle"));

        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(feedback_text(&runtime), Some("activated"));
    }

    fn feedback_text<Bridge>(runtime: &SurfaceRuntime<Bridge, DemoMessage>) -> Option<&str>
    where
        Bridge: crate::runtime::RuntimeBridge<DemoMessage>,
    {
        runtime
            .surface()
            .find_widget(90)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
            .map(|widget| widget.text.as_str())
    }
}
