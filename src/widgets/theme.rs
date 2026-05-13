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
}
