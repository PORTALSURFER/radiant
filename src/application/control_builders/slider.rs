use crate::{
    application::{
        MappedWidget, ViewNode, default_slider_sizing, primary_style, view_node_from_widget,
    },
    runtime::WidgetMessageMapper,
    widgets::{SliderMessage, SliderWidget, WidgetProminence, WidgetSizing, WidgetStyle},
};

/// Builder for horizontal sliders that emit explicit host messages.
pub struct SliderBuilder {
    value: f32,
    style: Option<WidgetStyle>,
    sizing: Option<crate::layout::Vector2>,
    paints_focus: Option<bool>,
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

    /// Use compact toolbar-friendly slider sizing.
    pub fn compact(mut self) -> Self {
        self.sizing = Some(crate::layout::Vector2::new(92.0, 20.0));
        self
    }

    /// Control whether this slider paints focus affordances.
    pub fn paint_focus(mut self, paint: bool) -> Self {
        self.paints_focus = Some(paint);
        self
    }

    /// Emit a host message mapped from the normalized slider value.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut slider = SliderWidget::new(
            0,
            self.value,
            self.sizing
                .map(WidgetSizing::fixed)
                .unwrap_or_else(default_slider_sizing),
        );
        if let Some(paint) = self.paints_focus {
            slider.common.paint.paints_focus = paint;
        }
        let mut node = view_node_from_widget(MappedWidget::new(
            slider,
            WidgetMessageMapper::slider(move |message| match message {
                SliderMessage::ValueChanged { value } => map(value),
            }),
        ));
        node.style = self.style;
        node
    }
}

/// Build a horizontal normalized slider.
pub fn slider(value: f32) -> SliderBuilder {
    SliderBuilder {
        value,
        style: None,
        sizing: None,
        paints_focus: None,
    }
}

/// Build a horizontal normalized slider that maps value changes.
pub fn slider_mapped<Message: 'static>(
    value: f32,
    map: impl Fn(f32) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    slider(value).message(map)
}
