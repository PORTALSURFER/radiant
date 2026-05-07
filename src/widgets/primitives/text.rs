//! Reusable text and label primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, push_text_widget_paint};
use crate::widgets::contract::{Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

/// Text wrapping behavior for text-like widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextWrap {
    /// Keep text on one line and clip overflow.
    None,
    /// Wrap text to additional lines inside the assigned rect.
    Word,
}

/// Public label/text primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct TextWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Displayed text content.
    pub text: String,
    /// Wrapping policy used for intrinsic sizing and paint.
    pub wrap: TextWrap,
}

impl TextWidget {
    /// Build a label/text widget with a preferred intrinsic size.
    pub fn new(id: WidgetId, text: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.paint.paints_focus = false;
        Self {
            common,
            text: text.into(),
            wrap: TextWrap::None,
        }
    }
}

impl Widget for TextWidget {
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
        push_text_widget_paint(primitives, self, bounds, theme);
    }
}
