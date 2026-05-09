use crate::{
    gui::types::{Rect, Rgba8},
    widgets::{TextInputState, TextWrap, WidgetId, WidgetStyle},
};

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

/// Single-line text primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintTextRun {
    /// Widget that produced this text run.
    pub widget_id: WidgetId,
    /// Text content to paint.
    pub text: String,
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
    pub label: Option<String>,
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
    pub placeholder: Option<String>,
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
    /// Selection fill color.
    pub selection_color: Rgba8,
    /// Block caret fill color.
    pub caret_color: Rgba8,
    /// Whether the field currently owns keyboard focus.
    pub focused: bool,
}
