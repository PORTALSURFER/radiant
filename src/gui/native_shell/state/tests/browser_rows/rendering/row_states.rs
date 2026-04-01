use super::*;

fn row_fill_color(frame: &NativeViewFrame, rect: Rect) -> Rgba8 {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(fill) if fill.rect == rect => Some(fill.color),
            _ => None,
        })
        .expect("row should emit a fill rectangle")
}

fn row_label_color(frame: &NativeViewFrame, label: &str) -> Rgba8 {
    frame
        .text_runs
        .iter()
        .find_map(|run| (run.text == label).then_some(run.color))
        .expect("row label should render")
}

fn color_luma(color: Rgba8) -> u16 {
    ((u16::from(color.r) * 54) + (u16::from(color.g) * 183) + (u16::from(color.b) * 19)) / 256
}

#[test]
fn browser_rows_use_alternating_fill_stripes_for_readability() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_even", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "row_odd", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(rendered.len() >= 2);

    let frame = state.build_frame(&layout, &model);
    let even_rect = rendered[0].rect;
    let odd_rect = rendered[1].rect;
    let even_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == even_rect => Some(rect.color),
            _ => None,
        })
        .collect();
    let odd_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == odd_rect => Some(rect.color),
            _ => None,
        })
        .collect();
    let expected_even = browser_row_stripe_fill(&style, 0);
    let expected_odd = browser_row_stripe_fill(&style, 1);
    assert!(!even_fills.is_empty(), "missing even-row fills");
    assert!(!odd_fills.is_empty(), "missing odd-row fills");
    assert!(
        even_fills.contains(&expected_even),
        "expected even-row fill {expected_even:?}, saw {even_fills:?}"
    );
    assert!(
        odd_fills.contains(&expected_odd),
        "expected odd-row fill {expected_odd:?}, saw {odd_fills:?}"
    );
    assert_ne!(expected_even, expected_odd);
}

#[test]
fn locked_browser_rows_keep_neutral_fill_and_draw_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "locked row", 1, false, false).with_locked(true));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("locked marker");
    let frame = state.build_frame(&layout, &model);
    let row_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == row.rect => Some(rect.color),
            _ => None,
        })
        .collect();

    assert!(
        row_fills.contains(&browser_row_stripe_fill(&style, 0)),
        "locked row should keep the standard stripe fill"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.accent_mint
        )
    }));
}

#[test]
fn marked_browser_rows_use_distinct_fill_and_draw_cyan_left_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "marked row", 1, false, false).with_marked(true));

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marker_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("marked marker");
    let frame = state.build_frame(&layout, &model);
    let row_fills: Vec<Rgba8> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == row.rect => Some(rect.color),
            _ => None,
        })
        .collect();

    assert!(
        row_fills.contains(&browser_marked_row_fill(&style, 0)),
        "marked row should render the dedicated marked fill"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marker_rect && *color == style.highlight_cyan
        )
    }));
}

#[test]
fn marked_locked_browser_rows_offset_keep_lock_marker_after_mark_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "marked locked row", 1, false, false)
            .with_marked(true)
            .with_locked(true),
    );

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let marked_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 0.0).expect("marked marker");
    let locked_rect =
        browser_locked_marker_rect(row.rect, style.sizing, 4.0).expect("locked marker");
    let frame = state.build_frame(&layout, &model);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == marked_rect && *color == style.highlight_cyan
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == locked_rect && *color == style.accent_mint
        )
    }));
}

#[test]
fn focused_browser_rows_render_similarity_button_on_far_left() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "focused row", 1, true, true));

    let button_rect = state
        .browser_similarity_button_rect(&layout, &model)
        .expect("focused row should expose a similarity button");
    let row = rendered_browser_rows(&layout, &model, &style)
        .into_iter()
        .next()
        .expect("browser row should render");
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let frame = state.build_frame(&layout, &model);

    assert!(
        button_rect.min.x <= row_text_layout.sample_label.min.x,
        "similarity button should stay on the far left edge of the sample column"
    );
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(FillRect { rect, .. }) if *rect == button_rect)
    }));
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| { matches!(primitive, Primitive::Image(DrawImage { .. })) })
    );
    assert!(!frame.text_runs.iter().any(|run| run.text == "SIM"));
}

#[test]
fn similarity_filtered_browser_rows_use_highlighted_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.similarity_filtered = true;
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "anchor", 1, true, true));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "match", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let anchor_rect = rendered[0].rect;
    let match_rect = rendered[1].rect;
    let anchor_fill = browser_similarity_row_fill(&style, 0, true);
    let match_fill = browser_similarity_row_fill(&style, 1, false);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == anchor_rect && *color == anchor_fill
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == match_rect && *color == match_fill
        )
    }));
}

#[test]
fn browser_playback_age_buckets_apply_distinct_row_fill_and_text_tints() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Fresh row", 1, false, false));
    model.browser.rows.push(
        BrowserRowModel::new(1, "Week row", 1, false, false)
            .with_playback_age_bucket(crate::app::PlaybackAgeBucket::OlderThanWeek),
    );
    model.browser.rows.push(
        BrowserRowModel::new(2, "Month row", 1, false, false)
            .with_playback_age_bucket(crate::app::PlaybackAgeBucket::OlderThanMonth),
    );
    model.browser.rows.push(
        BrowserRowModel::new(3, "Never row", 1, false, false)
            .with_playback_age_bucket(crate::app::PlaybackAgeBucket::NeverPlayed),
    );
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let fresh_fill = row_fill_color(&frame, rendered[0].rect);
    let week_fill = row_fill_color(&frame, rendered[1].rect);
    let month_fill = row_fill_color(&frame, rendered[2].rect);
    let never_fill = row_fill_color(&frame, rendered[3].rect);
    let fresh_text = row_label_color(&frame, "Fresh row");
    let week_text = row_label_color(&frame, "Week row");
    let month_text = row_label_color(&frame, "Month row");
    let never_text = row_label_color(&frame, "Never row");

    let expected_week_fill = age_browser_row_fill(
        &style,
        browser_row_stripe_fill(&style, 1),
        crate::app::PlaybackAgeBucket::OlderThanWeek,
    );
    let expected_month_fill = age_browser_row_fill(
        &style,
        browser_row_stripe_fill(&style, 2),
        crate::app::PlaybackAgeBucket::OlderThanMonth,
    );
    let expected_never_fill = age_browser_row_fill(
        &style,
        browser_row_stripe_fill(&style, 3),
        crate::app::PlaybackAgeBucket::NeverPlayed,
    );
    let expected_week_text = age_browser_row_text_color(
        &style,
        style.text_primary,
        crate::app::PlaybackAgeBucket::OlderThanWeek,
    );
    let expected_month_text = age_browser_row_text_color(
        &style,
        style.text_primary,
        crate::app::PlaybackAgeBucket::OlderThanMonth,
    );
    let expected_never_text = age_browser_row_text_color(
        &style,
        style.text_primary,
        crate::app::PlaybackAgeBucket::NeverPlayed,
    );

    assert_eq!(fresh_fill, browser_row_stripe_fill(&style, 0));
    assert_eq!(week_fill, expected_week_fill);
    assert_eq!(month_fill, expected_month_fill);
    assert_eq!(never_fill, expected_never_fill);
    assert_eq!(fresh_text, style.text_primary);
    assert_eq!(week_text, expected_week_text);
    assert_eq!(month_text, expected_month_text);
    assert_eq!(never_text, expected_never_text);

    assert_ne!(fresh_fill, week_fill);
    assert_ne!(fresh_fill, month_fill);
    assert_ne!(fresh_fill, never_fill);
    assert_ne!(week_fill, month_fill);
    assert_ne!(week_fill, never_fill);
    assert_ne!(month_fill, never_fill);
    assert_ne!(fresh_text, week_text);
    assert_ne!(fresh_text, month_text);
    assert_ne!(fresh_text, never_text);
    assert_ne!(week_text, month_text);
    assert_ne!(week_text, never_text);
    assert_ne!(month_text, never_text);

    for (fill, text) in [
        (week_fill, week_text),
        (month_fill, month_text),
        (never_fill, never_text),
    ] {
        assert!(
            color_luma(text) > color_luma(fill) + 40,
            "expected readable text contrast between {fill:?} and {text:?}"
        );
    }
}

#[test]
fn selected_browser_rows_keep_playback_age_tint_in_selection_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "selected month row", 1, true, false)
            .with_playback_age_bucket(crate::app::PlaybackAgeBucket::OlderThanMonth),
    );

    let row_rect = rendered_browser_rows(&layout, &model, &style)[0].rect;
    state.sync_from_model(&model);
    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);

    let selected_fill =
        selected_browser_row_fill(&style, crate::app::PlaybackAgeBucket::OlderThanMonth);
    let fresh_selected_fill =
        selected_browser_row_fill(&style, crate::app::PlaybackAgeBucket::Fresh);

    assert_ne!(selected_fill, fresh_selected_fill);
    assert!(overlay.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == row_rect && *color == selected_fill
        )
    }));
}
