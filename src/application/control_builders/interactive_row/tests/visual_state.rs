use super::*;

#[test]
fn input_only_dense_row_policy_preserves_existing_underlay_visual_state() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Sample"))
            .selected(true)
            .dense_row_policy(DenseRowPolicy::new().activation_modifiers())
            .style(crate::widgets::WidgetStyle::subtle(
                crate::widgets::WidgetTone::Accent,
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(120)),
        "input-only dense-row policies should not clear existing selected chrome"
    );
}

#[test]
fn dense_row_policy_visual_state_overrides_only_configured_parts() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Sample"))
            .active_target(true)
            .dense_row_policy(DenseRowPolicy::selectable(true))
            .style(crate::widgets::WidgetStyle::subtle(
                crate::widgets::WidgetTone::Accent,
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(220)),
        "selected policy should preserve separately configured active-target chrome"
    );
}

#[test]
fn selectable_dense_row_policy_paints_selected_chrome() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Sample"))
            .dense_row_policy(DenseRowPolicy::selectable(true).style(
                crate::widgets::WidgetStyle::subtle(crate::widgets::WidgetTone::Accent),
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(120)),
        "selectable dense-row policy should paint the standard selected fill"
    );
}

#[test]
fn interactive_row_underlay_paints_visible_content() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Collection"))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let paints_label = frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Collection"),
    );

    assert!(
        paints_label,
        "interactive row underlay should paint visible content"
    );
}

#[test]
fn interactive_row_underlay_dense_chrome_paints_selected_state() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Collection"))
            .selected(true)
            .style(crate::widgets::WidgetStyle::subtle(
                crate::widgets::WidgetTone::Accent,
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(120)),
        "dense underlay should paint the selected row fill"
    );
    assert!(
        frame.paint_plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text == "Collection"),
        ),
        "dense underlay should keep app-owned visible content above the row chrome"
    );
}

#[test]
fn interactive_row_underlay_dense_chrome_paints_active_target_state() {
    let frame = UiSurface::new(
        interactive_row_underlay(text("Collection"))
            .tracked_drop_target(true, true)
            .dense_chrome()
            .style(crate::widgets::WidgetStyle::subtle(
                crate::widgets::WidgetTone::Accent,
            ))
            .mapped(|_| ())
            .size(140.0, 22.0)
            .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == crate::theme::ThemeTokens::default()
                .accent_mint
                .with_alpha(220)),
        "dense underlay should paint the active drop-target fill"
    );
}

#[test]
fn interactive_row_underlay_dense_chrome_accepts_custom_paint_parts() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0));
    let selected_hover = Rgba8::new(30, 90, 120, 180);
    let leading = Rgba8::new(255, 90, 40, 240);
    let trailing = Rgba8::new(220, 220, 220, 210);
    let outline = Rgba8::new(80, 190, 255, 235);
    let mut surface = interactive_row_underlay(text("Sample"))
        .input_id(773)
        .selected(true)
        .dense_chrome_palette(
            DenseRowPalette::new()
                .selected(Rgba8::new(12, 24, 36, 160))
                .selected_hovered(selected_hover),
        )
        .leading_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
            leading,
        ))
        .trailing_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::trailing(2.0),
            trailing,
        ))
        .outline(DenseRowOutlineStyle::new(0.5, outline, 1.5))
        .mapped(|_| ())
        .size(140.0, 22.0)
        .into_surface();

    surface.dispatch_widget_input(
        773,
        bounds,
        WidgetInput::pointer_move(Point::new(12.0, 8.0)),
    );
    let frame = surface.frame(bounds, &Default::default());

    assert!(
        frame
            .paint_plan
            .fill_rects_for_widget(773)
            .any(|fill| fill.color == selected_hover),
        "custom underlay palette should still resolve through retained row hover state"
    );
    assert!(
        frame
            .paint_plan
            .fill_rects_for_widget(773)
            .any(|fill| fill.color == leading
                && fill.rect.min.x == bounds.min.x + 1.0
                && fill.rect.width() == 3.0),
        "custom underlay chrome should paint the leading marker"
    );
    assert!(
        frame
            .paint_plan
            .fill_rects_for_widget(773)
            .any(|fill| fill.color == trailing
                && fill.rect.min.x == bounds.max.x - 3.0
                && fill.rect.width() == 2.0),
        "custom underlay chrome should paint the trailing marker"
    );
    assert!(
        frame
            .paint_plan
            .stroke_rects_for_widget(773)
            .any(|stroke| stroke.color == outline && stroke.width == 1.5),
        "custom underlay chrome should paint the outline"
    );
}
