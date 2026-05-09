//! Reusable card and panel primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    Widget, WidgetId, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

mod paint;

/// Public card/panel primitive for grouped content surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct CardWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
}

impl CardWidget {
    /// Build a non-interactive card descriptor with neutral panel styling.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.paint.paints_focus = false;
        common.paint.suppresses_container_hover = true;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        Self { common }
    }
}

impl Widget for CardWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_card_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting card or panel leaf node.
    pub fn card(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(CardWidget::new(id, sizing))
    }
}
