use super::super::ViewNode;
use crate::{
    application::{danger_style, primary_style},
    widgets::{WidgetProminence, WidgetStyle},
};

impl<Message> ViewNode<Message> {
    /// Apply an explicit widget style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Allow this styled container to show hover chrome.
    pub fn hoverable(mut self) -> Self {
        self.hoverable = true;
        self
    }

    /// Keep an interactive widget in hit testing without painting its own chrome.
    pub fn input_only(mut self) -> Self {
        self.input_only = true;
        self
    }

    /// Show a passive runtime tooltip while this widget is hovered.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Show a passive runtime tooltip when one is provided.
    pub fn tooltip_opt(mut self, tooltip: Option<impl Into<String>>) -> Self {
        if let Some(tooltip) = tooltip {
            self.tooltip = Some(tooltip.into());
        }
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
}
