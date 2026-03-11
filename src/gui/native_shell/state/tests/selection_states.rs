use super::*;

#[test]
fn source_row_selected_fill_is_translucent_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "selected source",
        "detail",
        true,
        false,
    ));

    let selected_row = *state
        .rendered_source_row_rects(&layout, &model)
        .first()
        .expect("source row should be rendered");
    let frame = state.build_frame(&layout, &model);

    let row_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
            _ => None,
        })
        .expect("selected source row should emit a fill rectangle");

    assert_eq!(
        row_color,
        translucent_overlay_color(
            style.bg_tertiary,
            style.grid_soft,
            style.state_selected_blend
        )
    );
    assert!(row_color.a < 255);
}

#[test]
fn browser_row_selected_fill_uses_lighter_neutral_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "selected row", 1, true, false));

    let selected_row = rendered_browser_rows(&layout, &model, &style)[0].rect;
    let frame = state.build_frame(&layout, &model);
    let row_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
            _ => None,
        })
        .expect("selected browser row should emit a fill rectangle");

    assert_eq!(row_color, selected_browser_row_fill(&style));
}

#[test]
fn browser_row_locked_fill_tints_selected_row_mint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked row", 1, true, false).with_locked(true));

    let selected_row = rendered_browser_rows(&layout, &model, &style)[0].rect;
    let frame = state.build_frame(&layout, &model);
    let row_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == selected_row => Some(rect.color),
            _ => None,
        })
        .expect("locked browser row should emit a fill rectangle");

    assert_eq!(
        row_color,
        locked_browser_row_fill(&style, selected_browser_row_fill(&style))
    );
}

#[test]
fn browser_row_text_revision_changes_when_locked_state_changes() {
    let unlocked = [BrowserRowModel::new(0, "row", 1, false, false)];
    let locked = [BrowserRowModel::new(0, "row", 1, false, false).with_locked(true)];

    assert_ne!(
        browser_row_text_revision(&unlocked),
        browser_row_text_revision(&locked)
    );
}

#[test]
fn browser_row_selected_state_does_not_draw_mint_border() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "selected row", 1, true, false));

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let mint_border = blend_color(
        style.accent_mint,
        style.text_primary,
        style.state_selected_blend,
    );
    let has_mint_top_border =
        state
            .build_frame(&layout, &model)
            .primitives
            .iter()
            .any(|primitive| match primitive {
                Primitive::Rect(rect) => {
                    rect.color == mint_border
                        && rect.rect.min.x == border_rect.min.x
                        && rect.rect.max.x == border_rect.max.x
                        && rect.rect.min.y == border_rect.min.y
                        && rect.rect.max.y == border_rect.min.y + stroke
                }
                _ => false,
            });

    assert!(
        !has_mint_top_border,
        "selected browser rows should rely on fill instead of mint borders"
    );
}

#[test]
fn browser_row_focused_state_draws_bottom_focus_border() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "focused row", 1, false, true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "next row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let focus_border = blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    );
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let has_focus_bottom_border = frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => {
            rect.color == focus_border
                && rect.rect.min.x == border_rect.min.x
                && rect.rect.max.x == border_rect.max.x
                && rect.rect.min.y == border_rect.max.y - stroke
                && rect.rect.max.y == border_rect.max.y
        }
        _ => false,
    });

    assert!(
        has_focus_bottom_border,
        "focused browser rows should render a full border highlight"
    );
}

#[test]
fn browser_row_focused_state_draws_left_focus_border() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "focused row", 1, false, true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "next row", 1, false, false));
    state.sync_from_model(&model);

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let focus_border = blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    );
    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let has_focus_left_border = frame.primitives.iter().any(|primitive| match primitive {
        Primitive::Rect(rect) => {
            rect.color == focus_border
                && rect.rect.min.x == border_rect.min.x
                && rect.rect.max.x == border_rect.min.x + stroke
                && rect.rect.min.y == border_rect.min.y
                && rect.rect.max.y == border_rect.max.y
        }
        _ => false,
    });

    assert!(
        has_focus_left_border,
        "focused browser rows should keep their left focus border highlight"
    );
}
