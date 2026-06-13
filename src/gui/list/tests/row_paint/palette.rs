use super::fixtures::*;

#[test]
fn dense_row_palette_sets_interaction_fills_together() {
    let palette = DenseRowPalette::new().interaction_fills(HOVERED, PRESSED);

    assert_eq!(palette.hovered, Some(HOVERED));
    assert_eq!(palette.pressed, Some(PRESSED));
    assert_eq!(palette.selected, None);
    assert_eq!(palette.active_target, None);
    assert_eq!(palette.candidate_hovered, None);
}

#[test]
fn dense_row_palette_conditionally_sets_interaction_fills() {
    let enabled = DenseRowPalette::new().interaction_fills_if(true, HOVERED, PRESSED);
    let disabled = DenseRowPalette::new().interaction_fills_if(false, HOVERED, PRESSED);

    assert_eq!(enabled.hovered, Some(HOVERED));
    assert_eq!(enabled.pressed, Some(PRESSED));
    assert_eq!(disabled.hovered, None);
    assert_eq!(disabled.pressed, None);
}

#[test]
fn dense_row_palette_resolves_from_theme_style() {
    let theme = ThemeTokens::default();
    let style = WidgetStyle::subtle(WidgetTone::Accent);
    let palette = dense_row_palette_from_style(&theme, style);

    assert_eq!(palette.selected, Some(theme.accent_mint.with_alpha(120)));
    assert_eq!(palette.hovered, Some(theme.text_primary.with_alpha(24)));
    assert_eq!(
        palette.active_target,
        Some(theme.accent_mint.with_alpha(220))
    );
    assert_eq!(palette.candidate_hovered, palette.hovered);
    assert!(palette.pressed.is_some());
}

#[test]
fn dense_row_palette_prominence_strengthens_state_alpha() {
    let theme = ThemeTokens::default();
    let subtle = dense_row_palette_from_style(&theme, WidgetStyle::subtle(WidgetTone::Accent));
    let normal = dense_row_palette_from_style(
        &theme,
        WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Normal),
    );
    let strong = dense_row_palette_from_style(&theme, WidgetStyle::strong(WidgetTone::Accent));

    assert!(subtle.selected.unwrap().a < normal.selected.unwrap().a);
    assert!(normal.selected.unwrap().a < strong.selected.unwrap().a);
    assert!(subtle.hovered.unwrap().a < normal.hovered.unwrap().a);
    assert!(normal.hovered.unwrap().a < strong.hovered.unwrap().a);
}

#[test]
fn dense_row_outline_and_tree_guide_resolve_from_style() {
    let theme = ThemeTokens::default();
    let style = WidgetStyle::subtle(WidgetTone::Warning);

    assert_eq!(
        dense_row_drop_outline_from_style(&theme, style),
        DenseRowOutlineStyle::new(0.5, theme.accent_warning.with_alpha(235), 1.5)
    );
    assert_eq!(
        dense_row_tree_guide_color(&theme, style),
        theme.accent_warning.with_alpha(152)
    );
}
