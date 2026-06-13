use crate::{
    application::{
        MappedWidget, ViewNode, danger_style, default_toggle_sizing, primary_style,
        view_node_from_widget,
    },
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{ToggleMessage, ToggleWidget, WidgetProminence, WidgetStyle},
};

/// Builder for toggles that emit explicit host messages.
pub struct ToggleBuilder {
    label: PaintText,
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
        let sizing = default_toggle_sizing(&self.label, self.compact);
        let mut node = view_node_from_widget(MappedWidget::new(
            ToggleWidget::new(0, self.label, sizing).with_checked(self.checked),
            WidgetMessageMapper::toggle(move |message| match message {
                ToggleMessage::ValueChanged { checked } => map(checked),
            }),
        ));
        node.style = self.style;
        node
    }
}

/// Build a toggle.
pub fn toggle(label: impl Into<String>, checked: bool) -> ToggleBuilder {
    ToggleBuilder {
        label: PaintText::from(label.into()),
        checked,
        compact: false,
        style: None,
    }
}

/// Build a compact checkbox.
pub fn checkbox(checked: bool) -> ToggleBuilder {
    ToggleBuilder {
        label: PaintText::default(),
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
