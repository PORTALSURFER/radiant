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
fn browser_virtualization_hit_test_maps_first_middle_last_rendered_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    for visible_row in 0..200 {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:03}"),
            1,
            false,
            visible_row == 120,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_visible_row = Some(120);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(rendered.len() > 2);
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
        // Simulate the stale controller state after focus updates in a
        // short visible list: the shell must continue from the rows already on
        // screen instead of snapping back to zero.
        model.browser.view_start_row = 0;

        let start = state
            .cached_browser_rows(&layout, &style, &model)
            .first()
            .map(|row| row.visible_row)
            .expect("browser viewport should render rows");
        assert_eq!(start, expected_start, "focused_visible_row={focused_visible_row}");
    }
}

#[test]
/// Hit-testing should return no row when pointer sits in an inter-row gap.
fn browser_row_hit_test_returns_none_inside_gap() {
    let column = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 320.0));
    let rows = build_stacked_rows(column, 4, 6.0, 24.0);
    let cached_rows = cached_browser_rows_from_rects(rows.as_slice());
    let point = Point::new(
        (column.min.x + column.max.x) * 0.5,
        rows[0].max.y + ((rows[1].min.y - rows[0].max.y) * 0.5),
    );
    assert_eq!(
        row_index_for_visible_rows(&cached_rows, point, column),
        None
    );
}

#[test]
/// Zero-gap row boundaries should resolve to the earlier row for stable selection.
fn browser_row_hit_test_zero_gap_boundary_prefers_previous_row() {
    let column = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 320.0));
    let rows = build_stacked_rows(column, 3, 0.0, 24.0);
    let cached_rows = cached_browser_rows_from_rects(rows.as_slice());
    let point = Point::new((column.min.x + column.max.x) * 0.5, rows[1].min.y);
    assert_eq!(
        row_index_for_visible_rows(&cached_rows, point, column),
        Some(0)
    );
}

#[test]
/// Constant-time row hit-testing should match linear scan semantics.
fn browser_row_hit_test_matches_linear_scan_semantics() {
    let column = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 320.0));
    let rows = build_stacked_rows(column, 8, 5.0, 20.0);
    let cached_rows = cached_browser_rows_from_rects(rows.as_slice());
    let sample_points = [21.0, 39.0, 43.0, 46.0, 80.0, 144.0, 312.0];
    for y in sample_points {
        let point = Point::new((column.min.x + column.max.x) * 0.5, y);
        let linear = cached_rows.iter().position(|row| row.rect.contains(point));
        assert_eq!(
            row_index_for_visible_rows(&cached_rows, point, column),
            linear
        );
    }
}

#[test]
/// Fractional stacked-row metrics should still snap every row to stable pixel geometry.
fn browser_rows_snap_vertical_geometry_to_pixels() {
    let column = Rect::from_min_max(Point::new(10.0, 20.25), Point::new(310.0, 220.25));
    let rows = build_stacked_rows(column, 6, 1.4, 15.8);
    assert!(!rows.is_empty());
    let expected_height = rows[0].height();
    for row in rows {
        assert!(
            (row.min.y - row.min.y.round()).abs() <= 0.001,
            "row min y {} should snap to the pixel grid",
            row.min.y
        );
        assert!(
            (row.max.y - row.max.y.round()).abs() <= 0.001,
            "row max y {} should snap to the pixel grid",
            row.max.y
        );
        assert!(
            (row.height() - expected_height).abs() <= 0.001,
            "row height {} should stay stable",
            row.height()
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

#[test]
fn browser_row_hit_test_is_disabled_when_map_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = browser_model_with_rows(600, 300);
    model.map.active = true;
    let point = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
    );
    let mut state = NativeShellState::new();
    assert_eq!(state.browser_row_at_point(&layout, &model, point), None);
}

#[test]
fn browser_inline_metadata_prefers_explicit_row_metadata() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Kick 01", 1, true, true).with_bucket_label("165 BPM"));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("165 BPM"))
    );
}

#[test]
fn browser_inline_metadata_tags_render_chip_backgrounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(0, "Kick 01", 1, true, true)
            .with_bucket_label("165 BPM · LOOP · LONG"),
    );
    let frame = state.build_frame(&layout, &model);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    let row = rendered.first().expect("browser row should render");
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let expected_chip_rects = browser_inline_tag_chip_rects(
        row_text_layout.sample_label,
        &row.bucket_label,
        0.0,
        style.sizing,
    );
    assert_eq!(expected_chip_rects.len(), 3);
    for rect in expected_chip_rects {
        assert!(frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect: primitive_rect, color })
                    if *primitive_rect == rect
                        && *color == blend_color(style.surface_overlay, style.bg_tertiary, 0.54)
            )
        }));
    }
    for label in ["165 BPM", "LOOP", "LONG"] {
        assert!(frame.text_runs.iter().any(|run| run.text == label));
    }
}

#[test]
fn browser_header_omits_bucket_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(24, 8);
    let frame = state.build_frame(&layout, &model);
    assert!(!frame.text_runs.iter().any(|run| run.text == "Bucket"));
}

#[test]
fn static_segments_include_browser_rows_when_list_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = browser_model_with_rows(120, 40);
    let mut segments = StaticFrameSegments::default();
    for segment in StaticFrameSegment::ALL {
        state.build_static_segment_with_style_into(
            &layout,
            &style,
            &model,
            None,
            segment,
            &mut segments,
        );
    }
    let rows_segment = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    let map_segment = segments.frame(StaticFrameSegment::MapPanel);
    assert!(!rows_segment.primitives.is_empty());
    assert!(!rows_segment.text_runs.is_empty());
    assert!(map_segment.primitives.is_empty());
}

#[test]
fn static_segments_include_map_panel_when_map_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = browser_model_with_rows(120, 40);
    model.map.active = true;
    model.map.summary = String::from("Map summary");
    model.map.selected_sample_id = Some(String::from("kick"));
    model.map.focused_sample_id = Some(String::from("kick"));
    model.map.points = std::sync::Arc::from(vec![crate::app::MapPointModel {
        sample_id: std::sync::Arc::<str>::from("kick"),
        x_milli: 512,
        y_milli: 480,
        cluster_id: Some(1),
    }]);
    let mut segments = StaticFrameSegments::default();
    for segment in StaticFrameSegment::ALL {
        state.build_static_segment_with_style_into(
            &layout,
            &style,
            &model,
            None,
            segment,
            &mut segments,
        );
    }
    let rows_segment = segments.frame(StaticFrameSegment::BrowserRowsWindow);
    let map_segment = segments.frame(StaticFrameSegment::MapPanel);
    assert!(rows_segment.primitives.is_empty());
    assert!(!map_segment.primitives.is_empty());
    assert!(!map_segment.text_runs.is_empty());
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
fn browser_rows_share_single_pixel_separator_between_adjacent_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_top", 1, false, false));
    model
        .browser
        .rows
        .push(BrowserRowModel::new(1, "row_bottom", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert_eq!(rendered.len(), 2);

    let stroke = browser_row_border_stroke(&layout);
    let second_border = browser_row_border_rect(rendered[1].rect, stroke);
    let separator_count = state
        .build_frame(&layout, &model)
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.color == style.border
                    && rect.rect.min.x == second_border.min.x
                    && rect.rect.max.x == second_border.max.x
                    && rect.rect.min.y == second_border.min.y
                    && rect.rect.max.y == second_border.min.y + stroke
            }
            _ => false,
        })
        .count();

    assert_eq!(separator_count, 1);
}

#[test]
fn browser_rows_do_not_draw_extra_left_frame_edge_when_unfocused() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_plain", 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let row = &rendered_browser_rows(&layout, &model, &style)[0];
    let stroke = browser_row_border_stroke(&layout);
    let border_rect = browser_row_border_rect(row.rect, stroke);
    let has_left_border = state
        .build_frame(&layout, &model)
        .primitives
        .iter()
        .any(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.color == style.border
                    && rect.rect.min.x == border_rect.min.x
                    && rect.rect.max.x == border_rect.min.x + stroke
                    && rect.rect.min.y == border_rect.min.y
                    && rect.rect.max.y == border_rect.max.y
            }
            _ => false,
        });

    assert!(
        !has_left_border,
        "unfocused browser rows should not add an inner left frame edge"
    );
}

#[test]
fn browser_table_header_does_not_draw_extra_left_frame_edge() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let stroke = style.sizing.border_width.max(1.0);
    let has_left_border = state
        .build_frame(&layout, &model)
        .primitives
        .iter()
        .any(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.color == style.border
                    && rect.rect.min.x == layout.browser_table_header.min.x
                    && rect.rect.max.x == layout.browser_table_header.min.x + stroke
                    && rect.rect.min.y == layout.browser_table_header.min.y
                    && rect.rect.max.y == layout.browser_table_header.max.y
            }
            _ => false,
        });

    assert!(
        !has_left_border,
        "browser table header should share the outer sidebar/content seam instead of repainting its own left edge"
    );
}

#[test]
fn missing_browser_rows_render_red_exclamation_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "row_missing", 1, false, false).with_missing(true));
    model.browser.visible_count = model.browser.rows.len();

    let frame = state.build_frame(&layout, &model);
    let has_marker = frame.text_runs.iter().any(|run| {
        run.text == BROWSER_MISSING_SAMPLE_MARKER
            && run.color == BROWSER_MISSING_SAMPLE_MARKER_COLOR
            && (run.font_size - style.sizing.font_body).abs() <= f32::EPSILON
    });
    assert!(has_marker, "missing row marker should be rendered in red");
}

#[test]
fn browser_row_label_truncation_uses_slotized_sample_width() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let label =
        String::from("ultra_long_sample_label_that_should_be_truncated_by_slotized_sample_width");
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, label.clone(), 1, false, false));
    model.browser.visible_count = model.browser.rows.len();

    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert_eq!(rendered.len(), 1);
    let row = &rendered[0];
    let row_text_layout = compute_browser_row_text_layout(row.rect, style.sizing);
    let sample_width = row_text_layout.sample_label.width().max(20.0);
    assert_eq!(
        row.label,
        truncate_to_width(&label, sample_width, style.sizing.font_body)
    );
}

#[test]
fn browser_row_truncation_cache_reuses_entries_across_row_cache_rebuilds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    for index in 0..8 {
        model.browser.rows.push(
            BrowserRowModel::new(
                index,
                format!("very_long_browser_label_{index}_for_truncation_cache"),
                1,
                false,
                false,
            )
            .with_bucket_label("meta_bucket_label_that_is_also_long"),
        );
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_visible_row = Some(0);
    let _ = state.cached_browser_rows(&layout, &style, &model);
    let first = state.browser_row_truncation_frame_counts();
    assert!(first.lookup_count > 0);
    assert_eq!(first.cache_hit_count, 0);
    assert!(first.cache_miss_count > 0);

    model.browser.selected_visible_row = Some(1);
    let _ = state.cached_browser_rows(&layout, &style, &model);
    let second = state.browser_row_truncation_frame_counts();
    assert!(second.lookup_count > 0);
    assert!(second.cache_hit_count > 0);
    assert_eq!(second.cache_miss_count, 0);
}

#[test]
fn browser_row_truncation_cache_invalidates_when_row_text_revision_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.browser.rows.push(
        BrowserRowModel::new(
            0,
            "very_long_browser_label_for_truncation_cache",
            1,
            false,
            false,
        )
        .with_bucket_label("bucket_label"),
    );
    model.browser.rows.push(
        BrowserRowModel::new(
            1,
            "another_very_long_browser_label_for_truncation_cache",
            1,
            false,
            false,
        )
        .with_bucket_label("bucket_label"),
    );
    model.browser.visible_count = model.browser.rows.len();
    let _ = state.cached_browser_rows(&layout, &style, &model);
    let _ = state.browser_row_truncation_frame_counts();

    model.browser.rows[0].label = String::from("updated_long_browser_label_for_cache_reset");
    let _ = state.cached_browser_rows(&layout, &style, &model);
    let second = state.browser_row_truncation_frame_counts();
    assert!(second.lookup_count > 0);
    assert_eq!(second.cache_hit_count, 0);
    assert!(second.cache_miss_count > 0);
}
