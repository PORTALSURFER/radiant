use super::*;

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
fn browser_playback_age_bucket_grays_out_row_fill_and_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Fresh row", 1, false, false));
    model.browser.rows.push(
        BrowserRowModel::new(1, "Never row", 1, false, false)
            .with_playback_age_bucket(crate::app::PlaybackAgeBucket::NeverPlayed),
    );
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let frame = state.build_frame(&layout, &model);
    let never_rect = rendered[1].rect;
    let expected_never_fill = age_browser_row_color(
        browser_row_stripe_fill(&style, 1),
        crate::app::PlaybackAgeBucket::NeverPlayed,
    );
    let expected_never_text = age_browser_row_color(
        style.text_primary,
        crate::app::PlaybackAgeBucket::NeverPlayed,
    );

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == never_rect && *color == expected_never_fill
        )
    }));
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text == "Never row" && run.color == expected_never_text)
    );
}
