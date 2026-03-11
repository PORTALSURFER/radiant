//! Browser toolbar layout and hover helpers.

use super::super::*;

pub(in crate::gui::native_shell::state) fn browser_toolbar_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    browser_buttons: &[ActionButton],
) -> BrowserToolbarLayout {
    let action_left = browser_buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .or(Some(
            layout.browser_toolbar.max.x - style.sizing.text_inset_x,
        ));
    let sections =
        compute_browser_toolbar_sections(layout.browser_toolbar, style.sizing, action_left);
    BrowserToolbarLayout {
        rating_filter_chips: sections.rating_filter_chips,
        search_field: sections.search_field,
        activity_chip: sections.activity_chip,
        sort_chip: sections.sort_chip,
        triage_chips: sections.triage_chips,
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_index(
    level: i8,
) -> Option<usize> {
    BROWSER_RATING_FILTER_LEVELS
        .iter()
        .position(|chip| *chip == level)
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_level_at_point(
    chips: [Rect; 8],
    point: Point,
) -> Option<i8> {
    chips
        .iter()
        .position(|rect| rect.width() > 1.0 && rect.contains(point))
        .map(|index| BROWSER_RATING_FILTER_LEVELS[index])
}

pub(in crate::gui::native_shell::state) fn browser_column_chips(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    browser_buttons: &[ActionButton],
) -> Vec<BrowserColumnChip> {
    let _ = (layout, style, model, browser_buttons);
    Vec::new()
}

pub(in crate::gui::native_shell::state) fn render_browser_search_field_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    search_field_rect: Rect,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: search_field_rect,
            color: browser_search_field_hover_fill(style, motion_wave),
        }),
    );
    push_border(
        primitives,
        search_field_rect,
        browser_search_field_hover_border(style, motion_wave),
        sizing.border_width,
    );
}

pub(in crate::gui::native_shell::state) fn render_browser_rating_filter_chip_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    chip_rect: Rect,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: chip_rect,
            color: browser_rating_filter_chip_hover_fill(style, rating_level, active, motion_wave),
        }),
    );
    push_border(
        primitives,
        chip_rect,
        browser_rating_filter_chip_hover_border(style, rating_level, active, motion_wave),
        sizing.border_width,
    );
}

pub(in crate::gui::native_shell::state) fn browser_search_field_hover_fill(
    style: &StyleTokens,
    motion_wave: f32,
) -> Rgba8 {
    translucent_overlay_color(
        style.surface_base,
        style.bg_tertiary,
        0.22 + (motion_wave * 0.04),
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_hover_fill(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level > 0 {
        style.accent_mint
    } else {
        style.highlight_orange_soft
    };
    let amount = if active { 0.34 } else { 0.2 } + (motion_wave * 0.04);
    translucent_overlay_color(
        browser_rating_filter_chip_fill(style, rating_level, active),
        tint,
        amount,
    )
}

pub(in crate::gui::native_shell::state) fn browser_search_field_hover_border(
    style: &StyleTokens,
    motion_wave: f32,
) -> Rgba8 {
    blend_color(
        style.border_emphasis,
        style.text_primary,
        0.48 + (motion_wave * 0.06),
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_hover_border(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level > 0 {
        style.accent_mint
    } else {
        style.highlight_orange
    };
    blend_color(
        browser_rating_filter_chip_border(style, rating_level, active),
        tint,
        0.52 + (motion_wave * 0.08),
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_fill(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level == 4 {
        blend_color(style.accent_mint, style.text_primary, 0.28)
    } else if rating_level > 0 {
        style.accent_mint
    } else if active {
        style.highlight_orange
    } else {
        style.text_primary
    };
    let amount = if active {
        0.9
    } else if rating_level == 0 {
        0.14
    } else {
        0.18
    };
    blend_color(
        if active {
            style.surface_overlay
        } else {
            style.surface_base
        },
        tint,
        amount,
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_border(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
) -> Rgba8 {
    if active {
        if rating_level < 0 {
            blend_color(style.accent_trash, style.text_primary, 0.24)
        } else if rating_level == 4 {
            blend_color(style.accent_mint, style.text_primary, 0.44)
        } else if rating_level > 0 {
            blend_color(style.accent_mint, style.text_primary, 0.24)
        } else {
            blend_color(style.highlight_orange, style.text_primary, 0.22)
        }
    } else {
        blend_color(style.border, style.surface_overlay, 0.25)
    }
}

pub(in crate::gui::native_shell::state) fn browser_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let _ = (layout, style, model);
    Vec::new()
}
