use super::*;

#[test]
fn browser_virtualization_keeps_focused_row_visible_in_dense_column() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    for visible_row in 0..200 {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:03}"),
            1,
            false,
            visible_row == 150,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(150);
    model.browser.anchor_visible_row = Some(148);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(!rendered.is_empty());
    assert!(rendered.iter().any(|row| row.visible_row == 150));
    assert!(rendered.first().is_some_and(|first| first.visible_row > 0));
}

#[test]
fn browser_virtualization_clamps_tail_without_dropping_last_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = browser_model_with_rows(1000, 999);
    model.browser.selected_visible_row = Some(999);
    model.browser.anchor_visible_row = Some(996);

    let rendered = rendered_browser_rows(&layout, &model, &style);
    let expected_len = browser_rows_capacity(layout.browser_rows, style.sizing)
        .min(model.browser.rows.len())
        .max(1);
    assert_eq!(rendered.len(), expected_len);
    assert_eq!(rendered.last().map(|row| row.visible_row), Some(999));
    assert!(rendered.iter().any(|row| row.visible_row == 999));
}

#[test]
fn browser_virtualization_keeps_host_window_start_for_prewindowed_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);
    let focused_visible_row = host_window_start + (projected_rows / 2);
    let mut model = AppModel::default();
    for offset in 0..projected_rows {
        let visible_row = host_window_start + offset;
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = 5_000;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row);

    let rendered = rendered_browser_rows(&layout, &model, &style);

    assert_eq!(
        rendered.first().map(|row| row.visible_row),
        Some(host_window_start)
    );
}

#[test]
fn browser_virtualization_scrolls_down_for_bottom_rows_in_prewindowed_slice() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);

    let build_model = |focused_visible_row: usize, view_start_row: usize| {
        let mut model = AppModel::default();
        for offset in 0..projected_rows {
            let visible_row = host_window_start + offset;
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = 5_000;
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row);
        model.browser.autoscroll = true;
        model.browser.view_start_row = view_start_row;
        model
    };

    let bottom_focus = host_window_start + row_capacity.saturating_sub(1);
    let bottom_model = build_model(bottom_focus, host_window_start);
    let scrolled_start = rendered_browser_rows(&layout, &bottom_model, &style)
        .first()
        .map(|row| row.visible_row)
        .expect("bottom viewport should render at least one row");
    assert!(scrolled_start > host_window_start);

    let interior_model = build_model(scrolled_start + 5, scrolled_start);
    let preserved_start = rendered_browser_rows(&layout, &interior_model, &style)
        .first()
        .map(|row| row.visible_row)
        .expect("interior viewport should render at least one row");
    assert_eq!(preserved_start, scrolled_start);
}

#[test]
fn browser_virtualization_preserves_autoscroll_viewport_with_stale_host_view_start() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);
    let mut state = NativeShellState::new();

    let build_model = |focused_visible_row: usize| {
        let mut model = AppModel::default();
        for offset in 0..projected_rows {
            let visible_row = host_window_start + offset;
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = 5_000;
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row);
        model.browser.autoscroll = true;
        model.browser.view_start_row = host_window_start;
        model
    };

    let bottom_focus = host_window_start + row_capacity.saturating_sub(1);
    let bottom_model = build_model(bottom_focus);
    let scrolled_start = state
        .cached_browser_rows(&layout, &style, &bottom_model)
        .first()
        .map(|row| row.visible_row)
        .expect("bottom viewport should render at least one row");
    assert!(scrolled_start > host_window_start);

    let interior_model = build_model(scrolled_start + (row_capacity / 2));
    let preserved_start = state
        .cached_browser_rows(&layout, &style, &interior_model)
        .first()
        .map(|row| row.visible_row)
        .expect("interior viewport should render at least one row");
    assert_eq!(preserved_start, scrolled_start);
}

#[test]
fn browser_virtualization_keeps_center_focus_stable_in_scrolled_prewindowed_slice() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);
    let scrolled_view_start = host_window_start + (row_capacity / 2);
    let focused_visible_row = scrolled_view_start + (row_capacity / 2);
    let mut model = AppModel::default();
    for offset in 0..projected_rows {
        let visible_row = host_window_start + offset;
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = 5_000;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row);
    model.browser.autoscroll = true;
    model.browser.view_start_row = scrolled_view_start;

    let rendered = rendered_browser_rows(&layout, &model, &style);

    assert_eq!(
        rendered.first().map(|row| row.visible_row),
        Some(scrolled_view_start)
    );
    assert!(
        rendered
            .iter()
            .any(|row| row.visible_row == focused_visible_row)
    );
}

#[test]
fn browser_virtualization_hit_test_is_stable_across_viewport_tiers() {
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let mut state = NativeShellState::new();
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let model = browser_model_with_rows(1200, 940);
        let rendered = rendered_browser_rows(&layout, &model, &style);
        assert!(!rendered.is_empty());
        assert!(rendered.iter().any(|row| row.visible_row == 940));
        let middle = rendered.len() / 2;
        for index in [0, middle, rendered.len() - 1] {
            let row = &rendered[index];
            let point = Point::new(
                (row.rect.min.x + row.rect.max.x) * 0.5,
                (row.rect.min.y + row.rect.max.y) * 0.5,
            );
            assert_eq!(
                state.browser_row_at_point(&layout, &model, point),
                Some(row.visible_row)
            );
        }
    }
}

#[test]
fn browser_window_start_keeps_interior_focus_in_full_visible_slice_after_down_scroll() {
    let rows: Vec<_> = (0..40)
        .map(|visible_row| {
            BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 18,
            )
        })
        .collect();

    assert_eq!(
        browser_window_start_with_previous(&rows, 21, 40, Some(18), Some(18), true, 0, Some(1)),
        1
    );
}

#[test]
fn browser_window_start_keeps_interior_focus_in_full_visible_slice_after_up_scroll() {
    let rows: Vec<_> = (0..40)
        .map(|visible_row| {
            BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 6,
            )
        })
        .collect();

    assert_eq!(
        browser_window_start_with_previous(&rows, 21, 40, Some(6), Some(6), true, 0, Some(3)),
        3
    );
}

#[test]
fn browser_window_start_applies_guard_band_across_full_scroll_range() {
    let rows: Vec<_> = (0..40)
        .map(|visible_row| {
            BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                false,
            )
        })
        .collect();
    let window_len = 21usize;
    let max_start = rows.len() - window_len;
    let edge_margin = 3usize;

    for previous_start in 0..=max_start {
        let window_end = previous_start + window_len;
        for focus_row in previous_start..window_end {
            let expected = if focus_row < previous_start + edge_margin {
                focus_row.saturating_sub(edge_margin)
            } else if focus_row >= window_end.saturating_sub(edge_margin) {
                focus_row
                    .saturating_add(edge_margin + 1)
                    .saturating_sub(window_len)
            } else {
                previous_start
            }
            .min(max_start);

            assert_eq!(
                browser_window_start_with_previous(
                    &rows,
                    window_len,
                    rows.len(),
                    Some(focus_row),
                    Some(focus_row),
                    true,
                    0,
                    Some(previous_start),
                ),
                expected,
                "previous_start={previous_start}, focus_row={focus_row}"
            );
        }
    }
}

#[test]
fn browser_virtualization_preserves_guard_band_across_repeated_scrolled_refocuses() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let expected_sequence = [
        (18usize, 1usize),
        (19usize, 2usize),
        (20usize, 3usize),
        (15usize, 3usize),
        (5usize, 2usize),
        (4usize, 1usize),
        (3usize, 0usize),
    ];

    for (focused_visible_row, expected_start) in expected_sequence {
        let mut model = AppModel::default();
        for visible_row in 0..40 {
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row);
        model.browser.autoscroll = true;
        model.browser.view_start_row = 0;

        let start = state
            .cached_browser_rows(&layout, &style, &model)
            .first()
            .map(|row| row.visible_row)
            .expect("browser viewport should render rows");
        assert_eq!(
            start, expected_start,
            "focused_visible_row={focused_visible_row}"
        );
    }
}

#[test]
fn large_dataset_frame_build_is_deterministic_across_tiers() {
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(5000, 4777);
    state.sync_from_model(&model);
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let frame_a = state.build_frame(&layout, &model);
        let frame_b = state.build_frame(&layout, &model);
        assert_eq!(frame_a, frame_b);
        assert!(
            frame_a
                .text_runs
                .iter()
                .any(|run| run.text.contains("row_"))
        );
    }
}

#[test]
fn browser_virtualization_5k_rows_keeps_focus_and_tail_visible() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = browser_model_with_rows(5000, 4999);
    model.browser.selected_visible_row = Some(4999);
    model.browser.anchor_visible_row = Some(4995);

    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(!rendered.is_empty());
    assert_eq!(rendered.last().map(|row| row.visible_row), Some(4999));
    assert!(
        rendered
            .iter()
            .any(|row| row.visible_row == 4999 && row.focused)
    );
}
