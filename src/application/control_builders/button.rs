use crate::{
    application::{
        MappedWidget, ViewNode, compatibility::StateAction, danger_style, default_button_sizing,
        primary_style, view_node_from_widget,
    },
    gui::types::Point,
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{
        ButtonMessage, ButtonWidget, DragHandleMessage, WidgetOutput, WidgetProminence, WidgetStyle,
    },
};

mod state_actions;

use state_actions::{
    click_or_secondary_action, click_or_secondary_at_action, click_secondary_at_or_drag_action,
    click_secondary_or_drag_action,
};
use std::sync::Arc;

/// Builder for buttons that can emit messages or mutate state directly.
pub struct ButtonBuilder {
    label: PaintText,
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
                ButtonMessage::Activate => Some(activate.clone()),
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
            click_or_secondary_action(message, Arc::clone(&primary), Arc::clone(&secondary))
        })
    }

    /// Mutate application state on primary activation or secondary/right
    /// activation with pointer position.
    pub fn on_click_or_secondary_at<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State, Point) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        self.secondary_clicks().mapped(move |message| {
            click_or_secondary_at_action(message, Arc::clone(&primary), Arc::clone(&secondary))
        })
    }

    /// Mutate application state on primary, secondary/right, or drag lifecycle messages.
    pub fn on_click_secondary_or_drag<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State) + Send + Sync + 'static,
        drag: impl Fn(&mut State, DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        let drag = Arc::new(drag);
        self.secondary_clicks().draggable().mapped(move |message| {
            click_secondary_or_drag_action(
                message,
                Arc::clone(&primary),
                Arc::clone(&secondary),
                Arc::clone(&drag),
            )
        })
    }

    /// Mutate application state on primary, secondary/right with pointer
    /// position, or drag lifecycle messages.
    pub fn on_click_secondary_at_or_drag<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State, Point) + Send + Sync + 'static,
        drag: impl Fn(&mut State, DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        let drag = Arc::new(drag);
        self.secondary_clicks().draggable().mapped(move |message| {
            click_secondary_at_or_drag_action(
                message,
                Arc::clone(&primary),
                Arc::clone(&secondary),
                Arc::clone(&drag),
            )
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
        hover_chrome_only: false,
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
    map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    button(label).mapped(map)
}
