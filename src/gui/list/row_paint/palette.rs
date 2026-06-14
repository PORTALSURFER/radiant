use super::paint::DenseRowOutlineStyle;
use crate::{
    gui::types::Rgba8,
    theme::ThemeTokens,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

/// Fill colors for generic dense-row state projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DenseRowPalette {
    /// Fill for the selected state.
    pub selected: Option<Rgba8>,
    /// Fill for the combined selected and pointer-hover state.
    pub selected_hovered: Option<Rgba8>,
    /// Fill for pointer hover.
    pub hovered: Option<Rgba8>,
    /// Fill for pointer press.
    pub pressed: Option<Rgba8>,
    /// Fill for a committed operation target.
    pub active_target: Option<Rgba8>,
    /// Fill for a hovered operation candidate.
    pub candidate_hovered: Option<Rgba8>,
}

impl DenseRowPalette {
    /// Build an empty dense-row palette.
    pub const fn new() -> Self {
        Self {
            selected: None,
            selected_hovered: None,
            hovered: None,
            pressed: None,
            active_target: None,
            candidate_hovered: None,
        }
    }

    /// Set the selected fill color.
    pub const fn selected(mut self, color: Rgba8) -> Self {
        self.selected = Some(color);
        self
    }

    /// Set the selected and pointer-hover fill color.
    pub const fn selected_hovered(mut self, color: Rgba8) -> Self {
        self.selected_hovered = Some(color);
        self
    }

    /// Set the hovered fill color.
    pub const fn hovered(mut self, color: Rgba8) -> Self {
        self.hovered = Some(color);
        self
    }

    /// Set the pressed fill color.
    pub const fn pressed(mut self, color: Rgba8) -> Self {
        self.pressed = Some(color);
        self
    }

    /// Set the hovered and pressed fill colors together.
    pub const fn interaction_fills(mut self, hovered: Rgba8, pressed: Rgba8) -> Self {
        self.hovered = Some(hovered);
        self.pressed = Some(pressed);
        self
    }

    /// Set the hovered and pressed fill colors together when `condition` is true.
    pub const fn interaction_fills_if(
        mut self,
        condition: bool,
        hovered: Rgba8,
        pressed: Rgba8,
    ) -> Self {
        if condition {
            self.hovered = Some(hovered);
            self.pressed = Some(pressed);
        }
        self
    }

    /// Set the committed operation-target fill color.
    pub const fn active_target(mut self, color: Rgba8) -> Self {
        self.active_target = Some(color);
        self
    }

    /// Set the hovered operation-candidate fill color.
    pub const fn candidate_hovered(mut self, color: Rgba8) -> Self {
        self.candidate_hovered = Some(color);
        self
    }
}

/// Resolve standard dense-row feedback colors from theme tokens and semantic style.
///
/// Use this for compact lists, trees, and sidebars that need the same hover,
/// pressed, selected, operation-target, and operation-candidate semantics
/// without repeating app-local RGBA constants.
pub fn dense_row_palette_from_style(theme: &ThemeTokens, style: WidgetStyle) -> DenseRowPalette {
    let emphasis = dense_row_emphasis_color(theme, style.tone);
    let hover = dense_row_hover_fill(theme, style.prominence);
    DenseRowPalette::new()
        .selected(emphasis.with_alpha(dense_row_selected_alpha(style.prominence)))
        .interaction_fills(
            hover,
            dense_row_pressed_fill(theme, emphasis, style.prominence),
        )
        .active_target(emphasis.with_alpha(dense_row_active_target_alpha(style.prominence)))
        .candidate_hovered(hover)
}

/// Resolve the standard dense-row drop-target outline for a semantic style.
pub fn dense_row_drop_outline_from_style(
    theme: &ThemeTokens,
    style: WidgetStyle,
) -> DenseRowOutlineStyle {
    DenseRowOutlineStyle::new(
        0.5,
        dense_row_emphasis_color(theme, style.tone).with_alpha(235),
        1.5,
    )
}

/// Resolve the standard tree-guide color for dense tree rows.
pub fn dense_row_tree_guide_color(theme: &ThemeTokens, style: WidgetStyle) -> Rgba8 {
    dense_row_emphasis_color(theme, style.tone).with_alpha(match style.prominence {
        WidgetProminence::Subtle => 152,
        WidgetProminence::Normal => 176,
        WidgetProminence::Strong => 200,
    })
}

fn dense_row_emphasis_color(theme: &ThemeTokens, tone: WidgetTone) -> Rgba8 {
    match tone {
        WidgetTone::Neutral => theme.border_emphasis,
        WidgetTone::Accent => theme.accent_mint,
        WidgetTone::Success => theme.highlight_cyan,
        WidgetTone::Warning => theme.accent_warning,
        WidgetTone::Danger => theme.accent_danger,
    }
}

fn dense_row_hover_fill(theme: &ThemeTokens, prominence: WidgetProminence) -> Rgba8 {
    theme.text_primary.with_alpha(match prominence {
        WidgetProminence::Subtle => 24,
        WidgetProminence::Normal => 36,
        WidgetProminence::Strong => 48,
    })
}

fn dense_row_pressed_fill(
    theme: &ThemeTokens,
    emphasis: Rgba8,
    prominence: WidgetProminence,
) -> Rgba8 {
    let amount = match prominence {
        WidgetProminence::Subtle => 0.18,
        WidgetProminence::Normal => 0.12,
        WidgetProminence::Strong => 0.06,
    };
    emphasis
        .blend_opaque_toward(theme.text_primary, amount)
        .with_alpha(match prominence {
            WidgetProminence::Subtle => 170,
            WidgetProminence::Normal => 190,
            WidgetProminence::Strong => 210,
        })
}

fn dense_row_selected_alpha(prominence: WidgetProminence) -> u8 {
    match prominence {
        WidgetProminence::Subtle => 120,
        WidgetProminence::Normal => 150,
        WidgetProminence::Strong => 180,
    }
}

fn dense_row_active_target_alpha(prominence: WidgetProminence) -> u8 {
    match prominence {
        WidgetProminence::Subtle => 220,
        WidgetProminence::Normal => 230,
        WidgetProminence::Strong => 240,
    }
}
