//! Reusable text and label primitive.

use crate::gui::types::{Rect, Rgba8};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

mod builders;
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

/// Semantic foreground color for text-like widgets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextColorRole {
    /// Use the theme's primary text color.
    #[default]
    Primary,
    /// Use the theme's muted text color.
    Muted,
    /// Use the foreground color intended for text on strong accent fills.
    OnAccent,
    /// Use an explicit backend-neutral color.
    Custom(Rgba8),
}

/// Semantic background fill for text-like widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBackgroundRole {
    /// Use a strong accent fill suitable for inline suggestions and labels.
    Accent,
    /// Use an explicit backend-neutral color.
    Custom(Rgba8),
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
    /// Foreground color role used for paint.
    pub color: TextColorRole,
    /// Optional background fill role used for paint.
    pub background: Option<TextBackgroundRole>,
    /// Insets applied to the text rectangle inside the assigned bounds.
    pub inset: Vector2,
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
            color: TextColorRole::Primary,
            background: None,
            inset: Vector2::new(0.0, 0.0),
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

    /// Set the semantic foreground color role.
    pub fn with_color(mut self, color: TextColorRole) -> Self {
        self.color = color;
        self
    }

    /// Set an optional semantic background fill role.
    pub fn with_background(mut self, background: TextBackgroundRole) -> Self {
        self.background = Some(background);
        self
    }

    /// Set text insets inside the assigned widget bounds.
    pub fn with_inset(mut self, inset: Vector2) -> Self {
        self.inset = Vector2::new(inset.x.max(0.0), inset.y.max(0.0));
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

    fn set_text_color(&mut self, color: TextColorRole) -> bool {
        self.color = color;
        true
    }

    fn set_text_background(&mut self, background: TextBackgroundRole) -> bool {
        self.background = Some(background);
        true
    }

    fn set_text_inset(&mut self, inset: Vector2) -> bool {
        self.inset = Vector2::new(inset.x.max(0.0), inset.y.max(0.0));
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
