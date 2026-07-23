use crate::{
    application::{
        TextContent, ViewNode, danger_style, default_badge_sizing, primary_style,
        view_node_from_widget,
    },
    runtime::PaintText,
    widgets::{BadgeWidget, WidgetProminence, WidgetStyle},
};

/// Builder for badges and pills.
pub struct BadgeBuilder {
    pub(super) label: PaintText,
    pub(super) style: Option<WidgetStyle>,
    pub(super) active: bool,
    pub(super) outline: bool,
}

impl BadgeBuilder {
    /// Apply an explicit widget style before binding this badge.
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

    /// Mark this badge as active for visual state resolution.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Paint this badge as an outlined chip rather than a filled pill.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Build a passive badge view without host messages.
    pub fn passive<Message: 'static>(self) -> ViewNode<Message> {
        self.passive_view()
    }

    pub(super) fn passive_view<Message: 'static>(self) -> ViewNode<Message> {
        let sizing = default_badge_sizing(&self.label);
        let badge = BadgeWidget::new(0, self.label, sizing)
            .with_active(self.active)
            .with_outline(self.outline);
        let mut node = view_node_from_widget(badge);
        node.style = self.style;
        node
    }
}

/// Build a badge or pill.
pub fn badge(label: impl Into<TextContent>) -> BadgeBuilder {
    BadgeBuilder {
        label: label.into().into_paint_text(),
        style: None,
        active: false,
        outline: false,
    }
}
