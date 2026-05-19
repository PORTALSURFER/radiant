//! Reusable text and label primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText, SurfaceNode};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

mod paint;

/// Text wrapping behavior for text-like widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextWrap {
    /// Keep text on one line and clip overflow.
    None,
    /// Wrap text to additional lines inside the assigned rect.
    Word,
}

/// Horizontal alignment for label/text widgets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlign {
    /// Align text to the left edge of the assigned text rectangle.
    #[default]
    Left,
    /// Align text to the center of the assigned text rectangle.
    Center,
    /// Align text to the right edge of the assigned text rectangle.
    Right,
}

/// Public label/text primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct TextWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Displayed text content.
    pub text: PaintText,
    /// Wrapping policy used for intrinsic sizing and paint.
    pub wrap: TextWrap,
    /// Horizontal alignment used inside the assigned text rectangle.
    pub align: TextAlign,
}

/// Named construction fields for a [`TextWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextWidgetParts {
    /// Stable widget id used by layout and paint.
    pub id: WidgetId,
    /// Displayed text content.
    pub text: PaintText,
    /// Intrinsic sizing contract for the text widget.
    pub sizing: WidgetSizing,
}

impl TextWidget {
    /// Build a label/text widget from named parts.
    pub fn from_parts(parts: TextWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.paint.paints_focus = false;
        Self {
            common,
            text: parts.text,
            wrap: TextWrap::None,
            align: TextAlign::Left,
        }
    }

    /// Build a label/text widget with a preferred intrinsic size.
    pub fn new(id: WidgetId, text: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(TextWidgetParts {
            id,
            text: text.into(),
            sizing,
        })
    }

    /// Set horizontal alignment inside the assigned text rectangle.
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
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

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn set_text_wrap(&mut self, wrap: TextWrap) -> bool {
        self.wrap = wrap;
        true
    }

    fn set_text_align(&mut self, align: TextAlign) -> bool {
        self.align = align;
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_text_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting text leaf node.
    pub fn text(id: WidgetId, text: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::static_widget(TextWidget::new(id, text, sizing))
    }
}
