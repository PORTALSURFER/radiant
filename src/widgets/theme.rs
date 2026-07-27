//! Generic widget-theme helpers built on top of [`crate::theme`].
//!
//! These helpers let reusable widgets resolve a small visual treatment from the
//! core token surface without importing compatibility shell styling modules.

mod resolver;

#[cfg(test)]
mod tests;

pub use resolver::resolve_widget_visual_tokens;

use crate::gui::types::Rgba8;

/// Resolved generic widget colors for a specific theme, style, and state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetVisualTokens {
    /// Background fill for the widget body.
    pub fill: Rgba8,
    /// Text or icon foreground color.
    pub foreground: Rgba8,
    /// Border color around the widget body.
    pub border: Rgba8,
    /// Optional focus ring or selected outline color.
    pub emphasis: Rgba8,
    /// Non-color state cue for controls that need to remain distinguishable
    /// in monochrome or for color-impaired users.
    pub cue: WidgetVisualCue,
}

/// Precedence-resolved non-color cue for a widget's visual state.
///
/// Resolution order is `Disabled > Pressed > Focused > AutomationActive >
/// Selected > Active > Hovered > None`. Hosts may use this cue for a marker,
/// ring, hatch, or other shape treatment in addition to color tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WidgetVisualCue {
    /// No transient or semantic cue is active.
    #[default]
    None,
    /// The widget cannot currently be interacted with.
    Disabled,
    /// The primary action is pressed.
    Pressed,
    /// The widget has keyboard focus.
    Focused,
    /// Host automation currently owns or writes the widget.
    AutomationActive,
    /// The widget is selected.
    Selected,
    /// The widget is semantically active/on.
    Active,
    /// The pointer is hovering the widget.
    Hovered,
}
