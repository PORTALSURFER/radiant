use super::*;

#[test]
fn hovered_sections_do_not_emit_panel_fill_overlays() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    for hovered in [
        ShellNodeKind::TopBar,
        ShellNodeKind::Sidebar,
        ShellNodeKind::WaveformCard,
    ] {
        let mut frame = NativeViewFrame::default();
        state.hovered = Some(hovered);
        state.build_state_overlay_into(&layout, &style, &model, &mut frame);
        assert!(
            frame.primitives.iter().all(|primitive| {
                !matches!(
                    primitive,
                    Primitive::Rect(rect)
                        if rect.rect == layout.top_bar
                            || rect.rect == layout.sidebar
                            || rect.rect == layout.waveform_card
                )
            }),
            "hovered section should not emit a panel-sized fill overlay"
        );
    }
}

#[test]
fn browser_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.rows.push(BrowserRowModel::new(0, "hover", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "hover-2", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered_rows = rendered_browser_rows(&layout, &model, &style);
    let hover_row = rendered_rows[0].rect;
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = browser_row_hover_fill(&style);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
            _ => None,
        })
        .expect("hovered browser row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
}

#[test]
fn folder_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();

    let rendered_rows = rendered_folder_row_rects(&layout, &style, &model);
    let hover_row = rendered_rows[0];
    let cursor = Point::new(
        hover_row.min.x + 4.0,
        (hover_row.min.y + hover_row.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, cursor),
        CursorMoveEffect::None
    );

    let fingerprint = state.state_overlay_fingerprint();
    assert_eq!(fingerprint.hovered, Some(ShellNodeKind::Sidebar));
    assert_eq!(fingerprint.hovered_folder_row_index, Some(0));

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let expected_hover = subtle_item_hover_fill(&style);
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == hover_row => Some(rect.color),
            _ => None,
        })
        .expect("hovered folder row should emit a fill rectangle");

    assert_eq!(overlay_color, expected_hover);
}

#[test]
fn clearing_browser_row_hover_removes_unrelated_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(40, 18);
    let hovered_row = rendered_browser_rows(&layout, &model, &style)
        .into_iter()
        .find(|row| row.visible_row == 12)
        .map(|row| row.rect)
        .expect("hover target row should render");
    let hover_point = Point::new(
        hovered_row.min.x + 6.0,
        (hovered_row.min.y + hovered_row.max.y) * 0.5,
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, hover_point),
        CursorMoveEffect::None
    );
    assert_eq!(state.state_overlay_fingerprint().hovered_browser_visible_row, Some(12));

    state.clear_browser_row_hover();
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert_eq!(state.state_overlay_fingerprint().hovered_browser_visible_row, None);
    assert!(
        frame.primitives.iter().all(|primitive| {
            !matches!(
                primitive,
                Primitive::Rect(rect)
                    if rect.rect == hovered_row && rect.color == browser_row_hover_fill(&style)
            )
        }),
        "cleared browser row hover should remove the row-hover fill"
    );
}
