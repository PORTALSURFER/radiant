use crate::{
    gui::types::{Rect, Rgba8},
    widgets::{TextInputState, TextWrap, WidgetId, WidgetStyle},
};
use std::{fmt, ops::Deref, sync::Arc};

#[cfg(test)]
#[path = "text/tests.rs"]
mod tests;

/// Horizontal alignment for generic text paint primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintTextAlign {
    /// Align text to the left edge of the assigned text rectangle.
    Left,
    /// Align text to the center of the assigned text rectangle.
    Center,
    /// Align text to the right edge of the assigned text rectangle.
    Right,
}

/// Shared text payload used by backend-neutral paint primitives.
///
/// Paint plans are owned replayable artifacts, but most widget labels are
/// stable across frames. Sharing the text storage keeps repeated paint-plan
/// construction from reallocating every label string.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PaintText(Arc<str>);

impl PaintText {
    /// Return the text as a borrowed string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Return true when this text has no bytes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for PaintText {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<&str> for PaintText {
    fn from(value: &str) -> Self {
        Self(Arc::from(value))
    }
}

impl From<&String> for PaintText {
    fn from(value: &String) -> Self {
        Self(Arc::from(value.as_str()))
    }
}

impl AsRef<str> for PaintText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for PaintText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for PaintText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for PaintText {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<PaintText> for &str {
    fn eq(&self, other: &PaintText) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for PaintText {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<PaintText> for String {
    fn eq(&self, other: &PaintText) -> bool {
        self.as_str() == other.as_str()
    }
}

/// Single-line text primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextRun {
    /// Widget that produced this text run.
    pub widget_id: WidgetId,
    /// Text content to paint.
    pub text: PaintText,
    /// Text layout rectangle.
    pub rect: Rect,
    /// Font size in logical pixels per em.
    pub font_size: f32,
    /// Optional baseline measured from the text rectangle top edge.
    pub baseline: Option<f32>,
    /// Text color.
    pub color: Rgba8,
    /// Horizontal alignment inside the text rectangle.
    pub align: PaintTextAlign,
    /// Wrapping policy requested by the owning widget.
    pub wrap: TextWrap,
}

/// Floating overlay panel used for drag previews and transient popups.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintOverlayPanel {
    /// Stable overlay identifier.
    pub widget_id: WidgetId,
    /// Overlay rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Optional text label to paint inside the panel.
    pub label: Option<PaintText>,
    /// Panel style.
    pub style: WidgetStyle,
}

/// Single-line text-input primitive with native caret and selection state.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextInput {
    /// Widget that produced this text field.
    pub widget_id: WidgetId,
    /// Text layout rectangle inside the control chrome.
    pub rect: Rect,
    /// Optional placeholder shown when the value is empty.
    pub placeholder: Option<PaintText>,
    /// Optional inline completion text painted after the current value.
    pub completion_suffix: Option<PaintText>,
    /// Current text input state.
    pub state: TextInputState,
    /// Font size in logical pixels per em.
    pub font_size: f32,
    /// Optional baseline measured from the text rectangle top edge.
    pub baseline: Option<f32>,
    /// Value text color.
    pub color: Rgba8,
    /// Placeholder text color.
    pub placeholder_color: Rgba8,
    /// Inline completion text color.
    pub completion_color: Rgba8,
    /// Selection fill color.
    pub selection_color: Rgba8,
    /// Block caret fill color.
    pub caret_color: Rgba8,
    /// Whether the field currently owns keyboard focus.
    pub focused: bool,
}
