use super::super::{ViewNode, ViewNodeKind};
use crate::{application::OptionalBaseline, layout::Vector2, widgets::WidgetSizing};

impl<Message> ViewNode<Message> {
    /// Use explicit widget sizing instead of the generated default.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.sizing = Some(sizing);
        self
    }

    /// Use explicit fixed widget sizing instead of the generated default.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.sizing(WidgetSizing::fixed(Vector2::new(width, height)))
    }

    /// Use explicit fixed widget sizing instead of the generated default.
    pub fn fixed(self, width: f32, height: f32) -> Self {
        self.size(width, height)
    }

    /// Set the minimum widget size while preserving any existing preferred size.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        let min = Vector2::new(width, height);
        let preferred = self.sizing.map(|sizing| sizing.preferred).unwrap_or(min);
        let baseline = self.sizing.and_then(|sizing| sizing.baseline);
        self.sizing = Some(WidgetSizing::new(min, preferred).with_optional_baseline(baseline));
        self
    }

    /// Set the preferred widget size while preserving any existing minimum size.
    pub fn preferred_size(mut self, width: f32, height: f32) -> Self {
        let preferred = Vector2::new(width, height);
        let min = self.sizing.map(|sizing| sizing.min).unwrap_or(preferred);
        let baseline = self.sizing.and_then(|sizing| sizing.baseline);
        self.sizing = Some(WidgetSizing::new(min, preferred).with_optional_baseline(baseline));
        self
    }

    /// Set the widget text baseline.
    pub fn baseline(mut self, baseline: f32) -> Self {
        let sizing = self.sizing.unwrap_or_else(|| match &self.kind {
            ViewNodeKind::Widget(widget) => widget.default_sizing(),
            _ => WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        });
        self.sizing = Some(sizing.with_baseline(baseline));
        self
    }
}
