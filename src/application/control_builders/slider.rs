use crate::{
    application::{
        MappedWidget, StateAction, ViewNode, default_slider_sizing, primary_style,
        view_node_from_widget,
    },
    runtime::WidgetMessageMapper,
    widgets::{SliderMessage, SliderWidget, WidgetProminence, WidgetStyle},
};
use std::sync::Arc;

/// Builder for horizontal sliders that can emit messages or mutate state directly.
pub struct SliderBuilder {
    value: f32,
    style: Option<WidgetStyle>,
}

impl SliderBuilder {
    /// Apply an explicit widget style before binding this slider.
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

    /// Emit a host message mapped from the normalized slider value.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut node = view_node_from_widget(MappedWidget::new(
            SliderWidget::new(0, self.value, default_slider_sizing()),
            WidgetMessageMapper::slider(move |message| match message {
                SliderMessage::ValueChanged { value } => map(value),
            }),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when the slider value changes.
    pub fn on_change<State: 'static>(
        self,
        apply: impl Fn(&mut State, f32) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.message(move |value| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, value))
        })
    }
}

/// Build a horizontal normalized slider.
pub fn slider(value: f32) -> SliderBuilder {
    SliderBuilder { value, style: None }
}

/// Build a horizontal normalized slider that maps value changes.
pub fn slider_mapped<Message: 'static>(
    value: f32,
    map: impl Fn(f32) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    slider(value).message(map)
}
