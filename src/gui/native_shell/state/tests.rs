use super::*;
use crate::app::{
    BrowserRowModel, FolderActionsModel, FolderRowModel, NativeMotionModel, NormalizedRangeModel,
    SourceRowModel,
};
use crate::gui::types::{ImageRgba, Point, Rgba8, Vector2};

fn populated_sidebar_model() -> AppModel {
    let mut model = AppModel::default();
    for index in 0..20 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            format!("detail_{index:02}"),
            index == 2,
            false,
        ));
    }
    for index in 0..24 {
        model.sources.folder_rows.push(FolderRowModel::new(
            format!("folder_{index:02}"),
            String::new(),
            index % 4,
            false,
            index == 3,
            index == 0,
            true,
            true,
        ));
    }
    model.sources.folder_actions = FolderActionsModel {
        can_create_folder: true,
        can_create_folder_at_root: true,
        can_rename_folder: true,
        can_delete_folder: true,
        can_clear_recovery_log: true,
    };
    model
}

fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
    let mut model = AppModel::default();
    for visible_row in 0..total {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model
}

/// Build cached browser rows from rects for hit-test unit coverage.
fn cached_browser_rows_from_rects(rects: &[Rect]) -> Vec<CachedBrowserRow> {
    rects
        .iter()
        .copied()
        .enumerate()
        .map(|(index, rect)| CachedBrowserRow {
            visible_row: index,
            label: format!("row_{index}"),
            bucket_label: String::new(),
            column: 1,
            rating_level: 0,
            selected: false,
            focused: false,
            missing: false,
            rect,
        })
        .collect()
}

fn assert_rect_inside(outer: Rect, inner: Rect) {
    assert!(inner.min.x >= outer.min.x);
    assert!(inner.min.y >= outer.min.y);
    assert!(inner.max.x <= outer.max.x);
    assert!(inner.max.y <= outer.max.y);
}

fn assert_text_run_inside_band(run: &TextRun, band: Rect) {
    assert!(run.position.x >= band.min.x);
    assert!(run.position.x <= band.max.x);
    assert!(run.position.y >= band.min.y);
    assert!(run.position.y + run.font_size <= band.max.y + 0.5);
}

#[test]
fn browser_action_model_signature_changes_with_action_flags_and_chip_content() {
    let mut baseline = AppModel::default();
    baseline.browser_actions.can_rename = true;
    baseline.browser_actions.can_tag = true;
    baseline.browser_actions.can_delete = false;
    baseline.columns[0].title = String::from("Trash");
    baseline.columns[0].item_count = 10;
    baseline.columns[1].title = String::from("Neutral");
    baseline.columns[1].item_count = 20;
    baseline.columns[2].title = String::from("Keep");
    baseline.columns[2].item_count = 30;

    let baseline_signature = browser_action_model_signature(&baseline);

    let mut changed_flag = baseline.clone();
    changed_flag.browser_actions.can_delete = true;
    assert_ne!(
        baseline_signature,
        browser_action_model_signature(&changed_flag)
    );

    let mut changed_chip = baseline.clone();
    changed_chip.columns[2].title = String::from("Favorites");
    assert_ne!(
        baseline_signature,
        browser_action_model_signature(&changed_chip)
    );
}

#[test]
fn waveform_toolbar_model_flags_change_with_channel_and_toggle_state() {
    let baseline = NativeMotionModel::from_app_model(&AppModel::default());
    let baseline_flags = waveform_toolbar_model_flags(&baseline);

    let mut changed_channel = baseline.clone();
    changed_channel.waveform_channel_view = match baseline.waveform_channel_view {
        crate::app::WaveformChannelViewModel::Mono => crate::app::WaveformChannelViewModel::Stereo,
        crate::app::WaveformChannelViewModel::Stereo => crate::app::WaveformChannelViewModel::Mono,
    };
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_channel)
    );

    let mut changed_toggle = baseline.clone();
    changed_toggle.waveform_bpm_snap_enabled = !baseline.waveform_bpm_snap_enabled;
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_toggle)
    );
}

#[test]
fn sidebar_sections_keep_non_overlapping_bands_across_tiers() {
    let sizes = [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ];
    let mut state = NativeShellState::new();
    let model = populated_sidebar_model();
    for viewport in sizes {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let sections = sidebar_sections(&layout, &style, &model);
        let rendered_sources = state.rendered_source_row_rects(&layout, &model);
        assert_rect_inside(layout.sidebar_rows, sections.source_rows);
        assert_rect_inside(layout.sidebar_rows, sections.folder_header);
        assert_rect_inside(layout.sidebar_rows, sections.folder_rows);
        assert!(sections.source_rows.max.y <= sections.folder_header.min.y);
        assert!(sections.folder_header.max.y <= sections.folder_rows.min.y);
        assert!(!rendered_sources.is_empty());
    }
}

#[test]
fn sidebar_sections_remain_stable_in_cramped_viewports() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    assert_rect_inside(layout.sidebar_rows, sections.source_rows);
    assert_rect_inside(layout.sidebar_rows, sections.folder_header);
    assert_rect_inside(layout.sidebar_rows, sections.folder_rows);
    assert!(sections.source_rows.max.y <= sections.folder_header.min.y);
    assert!(sections.folder_header.max.y <= sections.folder_rows.min.y);
}

#[test]
fn waveform_deck_backplate_renders_inside_waveform_card() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);
    let backplate = waveform_deck_backplate_rect(layout.waveform_card, style.sizing);
    assert_rect_inside(layout.waveform_card, backplate);
    assert!(backplate.contains(layout.waveform_plot.min));
    assert!(backplate.contains(Point::new(
        layout.waveform_plot.max.x - 1.0,
        layout.waveform_plot.max.y - 1.0,
    )));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == backplate
                    && *color == blend_color(style.surface_overlay, style.bg_secondary, 0.38)
        )
    }));
}

#[test]
fn source_divider_remains_above_folder_rows_in_cramped_viewports() {
    let layout = ShellLayout::build(Vector2::new(820.0, 400.0));
    let style = style_for_layout(&layout);
    let model = populated_sidebar_model();
    let sections = sidebar_sections(&layout, &style, &model);
    let divider = compute_source_section_divider_rect(
        sections.source_rows,
        sections.folder_header,
        style.sizing,
    )
    .expect("divider should exist");
    assert_rect_inside(layout.sidebar_rows, divider);
    assert!(divider.max.y <= sections.folder_rows.min.y);
    assert!(divider.min.y >= sections.source_rows.min.y);
}

#[test]
fn folder_recovery_badge_compacts_label_when_header_is_narrow() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let style = style_for_layout(&layout);
    let header_rect = Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(58.0, style.sizing.folder_header_block_height),
    );
    let header_layout = compute_sidebar_folder_header_layout(header_rect, style.sizing, false, 153);
    let badge = header_layout.badge.expect("badge should still render");
    assert_rect_inside(header_rect, badge.rect);
    assert!(badge.label.chars().count() <= 3);
    assert!(!badge.active);
}

#[test]
fn folder_header_text_width_yields_no_overlap_with_recovery_badge() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let style = style_for_layout(&layout);
    let header_rect = Rect::from_min_max(
        Point::new(24.0, 40.0),
        Point::new(120.0, 40.0 + style.sizing.folder_header_block_height),
    );
    let header_layout = compute_sidebar_folder_header_layout(header_rect, style.sizing, true, 0);
    let badge = header_layout
        .badge
        .expect("badge should render for active recovery");
    assert!(header_layout.title_row.max.x <= badge.rect.min.x);
    if let Some(metadata_row) = header_layout.metadata_row {
        assert!(metadata_row.max.x <= badge.rect.min.x);
    }
}

#[test]
fn source_action_buttons_stay_inside_sidebar_footer() {
    let model = populated_sidebar_model();
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let buttons = source_action_buttons(&layout, &style, &model);
        assert!(!buttons.is_empty());
        for button in &buttons {
            assert_rect_inside(layout.sidebar_footer, button.rect);
        }
        for pair in buttons.windows(2) {
            assert!(pair[0].rect.max.x <= pair[1].rect.min.x);
        }
    }
}

#[test]
fn source_header_add_button_click_maps_to_add_source_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    assert_rect_inside(layout.sidebar_header, button);
    let point = Point::new(
        button.min.x + (button.width() * 0.5),
        button.min.y + (button.height() * 0.5),
    );
    let action = state
        .source_action_at_point(&layout, &model, point)
        .expect("source add button click should produce action");
    assert_eq!(action, UiAction::OpenOptionsMenu);
}

#[test]
fn browser_rating_indicator_layout_stays_inside_sample_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let keep = browser_rating_indicator_layout(row_text.sample_label, 3, sizing)
        .expect("keep indicators should render");
    let trash = browser_rating_indicator_layout(row_text.sample_label, -2, sizing)
        .expect("trash indicators should render");
    assert_eq!(keep.count, 3);
    assert_eq!(trash.count, 2);
    for rect in keep.rects.iter().take(keep.count) {
        assert_rect_inside(row_text.sample_label, *rect);
    }
    for rect in trash.rects.iter().take(trash.count) {
        assert_rect_inside(row_text.sample_label, *rect);
    }
    assert_eq!(browser_rating_indicator_color(&style, 3), style.accent_mint);
    assert_eq!(
        browser_rating_indicator_color(&style, -2),
        style.accent_trash
    );
}

#[test]
fn browser_action_buttons_stay_inside_toolbar() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_delete = true;
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let buttons = browser_action_buttons(&layout, &style, &model);
        assert!(buttons.is_empty());
    }
}

#[test]
fn browser_toolbar_controls_do_not_overlap_action_cluster() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_tag = true;
    model.browser_actions.can_delete = true;
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let buttons = browser_action_buttons(&layout, &style, &model);
    let controls = browser_toolbar_layout(&layout, &style, &buttons);
    assert!(buttons.is_empty());
    assert_rect_inside(layout.browser_toolbar, controls.search_field);
    assert!(controls.activity_chip.width() <= 0.0);
    assert!(controls.sort_chip.width() <= 0.0);
    assert!(
        controls
            .triage_chips
            .into_iter()
            .all(|chip| chip.width() <= 0.0)
    );
}

#[test]
fn top_bar_controls_fit_inside_control_row() {
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let controls = top_bar_controls_layout(&layout, style.sizing);
        if !controls.active {
            continue;
        }
        assert_rect_inside(layout.top_bar_controls_row, controls.options_label);
        assert_rect_inside(layout.top_bar_controls_row, controls.volume_meter);
        assert_rect_inside(layout.top_bar_controls_row, controls.volume_value);
        assert_rect_inside(layout.top_bar_controls_row, controls.volume_label);
        assert!(controls.options_label.max.x <= controls.volume_meter.min.x);
        assert!(controls.volume_meter.max.x <= controls.volume_value.min.x);
        assert!(controls.volume_value.max.x <= controls.volume_label.min.x);
    }
}

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
    model.browser.selected_visible_row = Some(150);
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
fn browser_virtualization_hit_test_is_stable_across_viewport_tiers() {
    let mut state = NativeShellState::new();
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
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
        browser_rating_indicator_reserved_width(row.rating_level, style.sizing),
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
    model.map.points.push(crate::app::MapPointModel {
        sample_id: String::from("kick"),
        x_milli: 512,
        y_milli: 480,
        cluster_id: Some(1),
        selected: true,
        focused: true,
    });
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
    assert!(
        map_segment
            .text_runs
            .iter()
            .any(|run| run.text.contains("Map"))
    );
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
    let even_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == even_rect => Some(rect.color),
            _ => None,
        });
    let odd_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == odd_rect => Some(rect.color),
            _ => None,
        });
    assert!(even_fill.is_some());
    assert!(odd_fill.is_some());
    assert_ne!(even_fill, odd_fill);
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

#[test]
fn waveform_title_uses_primary_text_hierarchy_color() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("WaveTitle"));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text == "WaveTitle" && run.color == style.text_primary)
    );
}

#[test]
fn waveform_image_data_emits_textured_waveform_primitive() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(1, 1, vec![11, 22, 33, 255]).unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let has_waveform_image = frame
        .primitives
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Image(_)));
    assert!(has_waveform_image);
}

#[test]
fn waveform_image_data_preserves_distinct_colors_in_texture_payload() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(
            1,
            2,
            vec![
                11, 22, 33, 255, // top pixel
                99, 88, 77, 255, // bottom pixel
            ],
        )
        .unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let (top_color_present, bottom_color_present) = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Image(image) => Some((
                image.image.pixels.get(0..4) == Some(&[11, 22, 33, 255]),
                image.image.pixels.get(4..8) == Some(&[99, 88, 77, 255]),
            )),
            _ => None,
        })
        .unwrap_or((false, false));
    assert!(top_color_present);
    assert!(bottom_color_present);
}

#[test]
fn waveform_image_transparent_pixels_do_not_emit_texture_primitive() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform.waveform_image = Some(std::sync::Arc::new(
        ImageRgba::new(1, 1, vec![11, 22, 33, 0]).unwrap(),
    ));
    let frame = state.build_frame(&layout, &model);
    let has_waveform_image = frame
        .primitives
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Image(_)));
    assert!(!has_waveform_image);
}

#[test]
fn map_header_prefers_projected_legend_selection_and_viewport_copy() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.map.active = true;
    model.map.legend_label = String::from("Render: points");
    model.map.selection_label = String::from("Selection: kick_24.wav");
    model.map.hover_label = String::from("Hover: kick_hover.wav");
    model.map.cluster_label = String::from("Clusters: 7");
    model.map.viewport_label = String::from("zoom 1.75x | pan (12, -8)");
    model.map.summary = String::from("248 points");

    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Render: points"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Selection: kick_24.wav"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Clusters: 7"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("zoom 1.75x | pan (12, -8)"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("248 points"))
    );
}

#[test]
fn map_header_metadata_stays_within_header_band() {
    let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.map.active = true;
    model.map.legend_label = String::from("Render: points");
    model.map.selection_label = String::from("Selection: very_long_sample_name.wav");
    model.map.cluster_label = String::from("Clusters: 42");

    let frame = state.build_frame(&layout, &model);
    let header_runs = frame
        .text_runs
        .iter()
        .filter(|run| run.text.contains("Render:") || run.text.contains("Selection:"))
        .collect::<Vec<_>>();
    assert!(!header_runs.is_empty());
    for run in header_runs {
        assert_text_run_inside_band(run, layout.browser_table_header);
    }
}

#[test]
fn hovered_top_bar_overlay_uses_hover_alpha() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let mut frame = NativeViewFrame::default();
    state.hovered = Some(ShellNodeKind::TopBar);

    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == layout.top_bar => Some(rect.color),
            _ => None,
        })
        .expect("hovered top bar should emit a fill rectangle");

    let expected_alpha =
        (style.sizing.hover_fill_alpha * (style.bg_tertiary.a as f32 / 255.0) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    assert_eq!(overlay_color.a, expected_alpha);
    assert_eq!(overlay_color.r, style.bg_tertiary.r);
    assert_eq!(overlay_color.g, style.bg_tertiary.g);
    assert_eq!(overlay_color.b, style.bg_tertiary.b);
    assert!(overlay_color.a < 255);
}

#[test]
fn browser_row_hovered_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "hover", 1, false, false));
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

    let expected_hover = subtle_item_hover_fill(&style);
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
fn cursor_move_tracks_waveform_hover_position_inside_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );
    let fingerprint = state.state_overlay_fingerprint();
    assert_eq!(fingerprint.hovered, Some(ShellNodeKind::WaveformCard));
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_some()
    );
}

#[test]
fn cursor_move_effect_classifies_waveform_hover_only_updates() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let first = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let second = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.7),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, first),
        CursorMoveEffect::GeneralOverlay
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, second),
        CursorMoveEffect::WaveformHoverOnly
    );
}

#[test]
fn cursor_move_effect_classifies_waveform_toolbar_hover_changes_as_general_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let mono_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Mono")
        .expect("mono button should be present");
    let stereo_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Stereo")
        .expect("stereo button should be present");
    let mono = Point::new(
        (mono_rect.min.x + mono_rect.max.x) * 0.5,
        (mono_rect.min.y + mono_rect.max.y) * 0.5,
    );
    let stereo = Point::new(
        (stereo_rect.min.x + stereo_rect.max.x) * 0.5,
        (stereo_rect.min.y + stereo_rect.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, mono),
        CursorMoveEffect::GeneralOverlay
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, stereo),
        CursorMoveEffect::GeneralOverlay
    );
}

#[test]
fn state_overlay_renders_waveform_toolbar_hover_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let mono_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Mono")
        .expect("mono button should be present");
    let mono = Point::new(
        (mono_rect.min.x + mono_rect.max.x) * 0.5,
        (mono_rect.min.y + mono_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, mono),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("View waveform in mono"))
    );
}

#[test]
fn cursor_move_clears_waveform_hover_position_outside_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let in_plot = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.4),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let outside = Point::new(
        layout.sidebar.min.x + 6.0,
        layout.sidebar.min.y + (layout.sidebar.height() * 0.5),
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, in_plot),
        CursorMoveEffect::None
    );
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_some()
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, outside),
        CursorMoveEffect::None
    );
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_none()
    );
}

#[test]
fn waveform_hover_overlay_draws_preview_cursor_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.6),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );
    let motion = NativeMotionModel::from_app_model(&model);
    let hover_x = f32::from_bits(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .expect("waveform hover should be tracked"),
    );
    let hover_marker_width = (style.sizing.border_width * 2.0).max(2.0);
    let expected_marker =
        waveform_hover_marker_rect(layout.waveform_plot, hover_marker_width, hover_x)
            .expect("cursor marker rect should exist");
    let expected_color = blend_color(style.accent_warning, style.text_primary, 0.72);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_marker => Some(rect.color),
            _ => None,
        })
        .expect("waveform hover marker should emit a cursor fill rectangle");
    assert_eq!(overlay_color, expected_color);
}

#[test]
fn waveform_motion_overlay_draws_distinct_play_and_edit_selection_marks() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let play_selection = NormalizedRangeModel::new(180, 420);
    let edit_selection = NormalizedRangeModel::new(560, 820);
    let edit_selection_blue = Rgba8 {
        r: 86,
        g: 156,
        b: 255,
        a: 255,
    };
    model.waveform.selection_milli = Some(play_selection);
    model.waveform.edit_selection_milli = Some(edit_selection);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let play_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(play_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("play selection rect");
    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");

    let play_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == play_rect => Some(rect.color),
            _ => None,
        })
        .expect("play selection fill");
    let edit_fill = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == edit_rect => Some(rect.color),
            _ => None,
        })
        .expect("edit selection fill");
    assert_eq!(
        play_fill,
        translucent_overlay_color(style.bg_secondary, style.accent_warning, 0.52)
    );
    assert_eq!(
        edit_fill,
        translucent_overlay_color(style.bg_secondary, edit_selection_blue, 0.5)
    );
    assert_ne!(play_fill, edit_fill);
}

#[test]
fn waveform_motion_overlay_omits_selection_resize_handles_until_hovered() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("selection rect");
    let left_edge_x = selection_rect.min.x;
    let right_edge_x = selection_rect.max.x;
    let selection_center_y = selection_rect.min.y + (selection_rect.height() * 0.5);

    let has_left_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= left_edge_x
                    && rect.rect.max.x >= left_edge_x
                    && rect.rect.min.y >= selection_rect.min.y
                    && rect.rect.max.y <= selection_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - selection_center_y).abs()
                        <= (selection_rect.height() * 0.05)
                    && rect.rect.height() < selection_rect.height()
        )
    });
    let has_right_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= right_edge_x
                    && rect.rect.max.x >= right_edge_x
                    && rect.rect.min.y >= selection_rect.min.y
                    && rect.rect.max.y <= selection_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - selection_center_y).abs()
                        <= (selection_rect.height() * 0.05)
                    && rect.rect.height() < selection_rect.height()
        )
    });
    assert!(
        !has_left_handle,
        "selection edges should not draw standalone handles"
    );
    assert!(
        !has_right_handle,
        "selection edges should not draw standalone handles"
    );
}

#[test]
fn waveform_motion_overlay_highlights_hovered_selection_resize_edge() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let selection = NormalizedRangeModel::new(180, 420);
    model.waveform.selection_milli = Some(selection);
    state.hovered_waveform_resize_edge = Some(WaveformResizeHoverEdge::SelectionStart);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("selection rect");
    let edge_x = selection_rect.min.x;
    let center_y = selection_rect.min.y + (selection_rect.height() * 0.5);

    let has_edge_highlight = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= edge_x
                    && rect.rect.max.x >= edge_x
                    && rect.rect.min.y >= selection_rect.min.y
                    && rect.rect.max.y <= selection_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - center_y).abs()
                        <= (selection_rect.height() * 0.05)
                    && rect.rect.height() < selection_rect.height()
        )
    });
    assert!(
        has_edge_highlight,
        "expected hovered selection edge highlight"
    );
}

#[test]
fn waveform_motion_overlay_draws_edit_fade_handles() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_out_start_milli = Some(690);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");
    let span = f32::from(edit_selection.end_milli - edit_selection.start_milli).max(1.0);
    let handle_in_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(320u16 - edit_selection.start_milli) / span));
    let handle_out_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(690u16 - edit_selection.start_milli) / span));

    let has_in_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_in_x
                    && rect.rect.max.x >= handle_in_x
                    && rect.rect.min.y <= edit_rect.min.y
                    && rect.rect.max.y >= edit_rect.max.y
        )
    });
    let has_out_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_out_x
                    && rect.rect.max.x >= handle_out_x
                    && rect.rect.min.y <= edit_rect.min.y
                    && rect.rect.max.y >= edit_rect.max.y
        )
    });
    assert!(has_in_handle, "expected edit fade-in handle primitive");
    assert!(has_out_handle, "expected edit fade-out handle primitive");
}

#[test]
fn waveform_motion_overlay_draws_edit_fade_top_grab_tabs() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_out_start_milli = Some(690);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");
    let span = f32::from(edit_selection.end_milli - edit_selection.start_milli).max(1.0);
    let handle_in_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(320u16 - edit_selection.start_milli) / span));
    let handle_out_x = edit_rect.min.x
        + (edit_rect.width() * (f32::from(690u16 - edit_selection.start_milli) / span));

    let has_in_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_in_x
                    && rect.rect.max.x >= handle_in_x
                    && rect.rect.min.y == edit_rect.min.y
                    && rect.rect.height() < edit_rect.height()
        )
    });
    let has_out_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_out_x
                    && rect.rect.max.x >= handle_out_x
                    && rect.rect.min.y == edit_rect.min.y
                    && rect.rect.height() < edit_rect.height()
        )
    });
    assert!(has_in_tab, "expected top grab tab for fade-in handle");
    assert!(has_out_tab, "expected top grab tab for fade-out handle");
}

#[test]
fn waveform_motion_overlay_draws_edit_fade_bottom_grab_tabs() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_in_mute_start_milli = Some(150);
    model.waveform.edit_fade_out_start_milli = Some(690);
    model.waveform.edit_fade_out_mute_end_milli = Some(860);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");
    let handle_in_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.15);
    let handle_out_x = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.86);

    let has_in_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_in_x
                    && rect.rect.max.x >= handle_in_x
                    && rect.rect.max.y == edit_rect.max.y
                    && rect.rect.height() < edit_rect.height()
                    && rect.rect.min.x < edit_rect.min.x
        )
    });
    let has_out_tab = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= handle_out_x
                    && rect.rect.max.x >= handle_out_x
                    && rect.rect.max.y == edit_rect.max.y
                    && rect.rect.height() < edit_rect.height()
                    && rect.rect.max.x > edit_rect.max.x
        )
    });
    assert!(has_in_tab, "expected bottom grab tab for fade-in handle");
    assert!(has_out_tab, "expected bottom grab tab for fade-out handle");
}

#[test]
fn waveform_motion_overlay_draws_edit_fade_curve_trace() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    model.waveform.edit_fade_in_end_milli = Some(320);
    model.waveform.edit_fade_in_mute_start_milli = Some(150);
    model.waveform.edit_fade_in_curve_milli = Some(800);
    model.waveform.edit_fade_out_start_milli = Some(690);
    model.waveform.edit_fade_out_mute_end_milli = Some(860);
    model.waveform.edit_fade_out_curve_milli = Some(250);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");
    let fade_in_right = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.32);
    let fade_out_left = layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.69);

    let has_left_curve_trace = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.width() <= 4.0
                    && rect.rect.height() <= 4.0
                    && rect.rect.max.x <= fade_in_right
                    && rect.rect.min.x < edit_rect.min.x
                    && rect.rect.min.x >= layout.waveform_plot.min.x
                    && rect.rect.min.y > edit_rect.min.y
                    && rect.rect.max.y < edit_rect.max.y
        )
    });
    let has_right_curve_trace = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.width() <= 4.0
                    && rect.rect.height() <= 4.0
                    && rect.rect.min.x >= fade_out_left
                    && rect.rect.max.x > edit_rect.max.x
                    && rect.rect.max.x <= layout.waveform_plot.max.x
                    && rect.rect.min.y > edit_rect.min.y
                    && rect.rect.max.y < edit_rect.max.y
        )
    });
    assert!(
        has_left_curve_trace,
        "expected fade-in curve markers past the selection start"
    );
    assert!(
        has_right_curve_trace,
        "expected fade-out curve markers past the selection end"
    );
}

#[test]
fn waveform_motion_overlay_omits_edit_resize_handles_until_hovered() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");
    let left_edge_x = edit_rect.min.x;
    let right_edge_x = edit_rect.max.x;
    let edit_center_y = edit_rect.min.y + (edit_rect.height() * 0.5);

    let has_left_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= left_edge_x
                    && rect.rect.max.x >= left_edge_x
                    && rect.rect.min.y >= edit_rect.min.y
                    && rect.rect.max.y <= edit_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - edit_center_y).abs()
                        <= (edit_rect.height() * 0.05)
                    && rect.rect.height() < edit_rect.height()
        )
    });
    let has_right_handle = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= right_edge_x
                    && rect.rect.max.x >= right_edge_x
                    && rect.rect.min.y >= edit_rect.min.y
                    && rect.rect.max.y <= edit_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - edit_center_y).abs()
                        <= (edit_rect.height() * 0.05)
                    && rect.rect.height() < edit_rect.height()
        )
    });
    assert!(
        !has_left_handle,
        "edit edges should not draw standalone handles"
    );
    assert!(
        !has_right_handle,
        "edit edges should not draw standalone handles"
    );
}

#[test]
fn waveform_motion_overlay_highlights_hovered_edit_resize_edge() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let edit_selection = NormalizedRangeModel::new(200, 800);
    model.waveform.edit_selection_milli = Some(edit_selection);
    state.hovered_waveform_resize_edge = Some(WaveformResizeHoverEdge::EditSelectionEnd);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let edit_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("edit selection rect");
    let edge_x = edit_rect.max.x;
    let center_y = edit_rect.min.y + (edit_rect.height() * 0.5);

    let has_edge_highlight = frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.rect.min.x <= edge_x
                    && rect.rect.max.x >= edge_x
                    && rect.rect.min.y >= edit_rect.min.y
                    && rect.rect.max.y <= edit_rect.max.y
                    && (rect.rect.min.y + (rect.rect.height() * 0.5) - center_y).abs()
                        <= (edit_rect.height() * 0.05)
                    && rect.rect.height() < edit_rect.height()
        )
    });
    assert!(has_edge_highlight, "expected hovered edit edge highlight");
}

#[test]
fn waveform_motion_overlay_draws_loop_range_bar_when_loop_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let play_selection = NormalizedRangeModel::new(260, 620);
    model.waveform.selection_milli = Some(play_selection);
    model.waveform.loop_enabled = true;
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let selection_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        Some(play_selection),
        None,
        None,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .selection
    .expect("play selection rect");
    let bar_height = 3.0f32
        .max(style.sizing.border_width)
        .min(selection_rect.height().max(1.0));
    let top = Rect::from_min_max(
        selection_rect.min,
        Point::new(
            selection_rect.max.x,
            (selection_rect.min.y + bar_height).min(selection_rect.max.y),
        ),
    );
    let bottom = Rect::from_min_max(
        Point::new(
            selection_rect.min.x,
            (selection_rect.max.y - bar_height).max(selection_rect.min.y),
        ),
        selection_rect.max,
    );
    let top_color = translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.42);
    let bottom_color = translucent_overlay_color(style.surface_overlay, style.accent_copper, 0.32);

    let has_top = frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect == top && rect.color == top_color)
    });
    let has_bottom = frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect == bottom && rect.color == bottom_color)
    });
    assert!(has_top, "expected top loop-range bar fill");
    assert!(has_bottom, "expected bottom loop-range bar fill");
}

#[test]
fn waveform_motion_overlay_draws_playhead_trail_when_transport_running() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    let mut frame = NativeViewFrame::default();
    for playhead in [700u16, 712, 724, 736, 748] {
        model.waveform.playhead_milli = Some(playhead);
        let motion = NativeMotionModel::from_app_model(&model);
        state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
    }

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");

    let trail_rect_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == playhead_rect.min.y
                    && rect.rect.max.y == playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();

    assert!(
        trail_rect_count >= 8,
        "expected many retained ghost lines, got {trail_rect_count}"
    );
}

#[test]
fn waveform_motion_overlay_omits_playhead_trail_when_playhead_is_stationary() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(740);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");

    let trail_rect_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == playhead_rect.min.y
                    && rect.rect.max.y == playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();

    assert_eq!(trail_rect_count, 0);
}

#[test]
fn waveform_motion_overlay_draws_backward_playhead_trail() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(740);
    let first_motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &first_motion, &mut frame);

    model.waveform.playhead_milli = Some(700);
    let second_motion = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &second_motion, &mut frame);

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");

    let trail_rect_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == playhead_rect.min.y
                    && rect.rect.max.y == playhead_rect.max.y
                    && rect.rect.min.x >= playhead_rect.max.x
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();

    assert!(
        trail_rect_count >= 3,
        "expected backward ghost lines, got {trail_rect_count}"
    );
}

#[test]
fn waveform_motion_overlay_clears_playhead_trail_when_transport_stops() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut frame = NativeViewFrame::default();
    model.transport_running = true;
    for playhead in [700u16, 718, 736, 754] {
        model.waveform.playhead_milli = Some(playhead);
        let motion = NativeMotionModel::from_app_model(&model);
        state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
    }

    let running_playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");

    let trail_rect_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == running_playhead_rect.min.y
                    && rect.rect.max.y == running_playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();
    assert!(
        trail_rect_count > 0,
        "expected running ghost lines before stop"
    );

    model.transport_running = false;
    model.waveform.playhead_milli = Some(754);
    let stopped_motion = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &stopped_motion, &mut frame);
    let stopped_playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");
    let cleared_trail_rect_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == stopped_playhead_rect.min.y
                    && rect.rect.max.y == stopped_playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();
    assert_eq!(cleared_trail_rect_count, 0);
}

#[test]
fn waveform_motion_overlay_clears_trail_on_large_playhead_jump() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut frame = NativeViewFrame::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(200);
    let first = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &first, &mut frame);
    model.waveform.playhead_milli = Some(240);
    let second = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &second, &mut frame);

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");
    let trail_before_jump = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == playhead_rect.min.y
                    && rect.rect.max.y == playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();
    assert!(trail_before_jump > 0, "expected baseline running trail");

    model.waveform.playhead_milli = Some(840);
    let jumped = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &jumped, &mut frame);
    let jumped_playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("jumped playhead marker");
    let trail_after_jump = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == jumped_playhead_rect.min.y
                    && rect.rect.max.y == jumped_playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();
    assert_eq!(trail_after_jump, 0);
}

#[test]
fn waveform_motion_overlay_omits_playhead_trail_when_transport_stopped_without_history() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = false;
    model.waveform.playhead_milli = Some(740);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_milli,
        model.waveform.view_end_milli,
    )
    .playhead
    .expect("playhead marker");

    let trail_rect_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == playhead_rect.min.y
                    && rect.rect.max.y == playhead_rect.max.y
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();

    assert_eq!(trail_rect_count, 0);
}

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
/// Source context menu hit testing should emit reload for the targeted row.
fn source_context_menu_hit_test_emits_reload_action_for_row() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    let row_rect = *state
        .rendered_source_row_rects(&layout, &model)
        .first()
        .expect("source row should be rendered");
    let anchor = Point::new(
        (row_rect.min.x + row_rect.max.x) * 0.5,
        (row_rect.min.y + row_rect.max.y) * 0.5,
    );
    state.open_source_context_menu_for_row(0, anchor);

    let reload_rect = state
        .source_context_menu_button_rect(&layout, &model, UiAction::ReloadSourceRow { index: 0 })
        .expect("reload action button should be present");
    let point = Point::new(
        (reload_rect.min.x + reload_rect.max.x) * 0.5,
        (reload_rect.min.y + reload_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.source_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::ReloadSourceRow { index: 0 })
    );
}

#[test]
/// Source context menu geometry should disappear after explicit close.
fn source_context_menu_contains_point_tracks_open_close_state() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    state.open_source_context_menu_for_row(
        0,
        Point::new(layout.sidebar.min.x + 24.0, layout.sidebar.min.y + 24.0),
    );
    let reload_rect = state
        .source_context_menu_button_rect(&layout, &model, UiAction::ReloadSourceRow { index: 0 })
        .expect("reload action button should be present");
    let point = Point::new(
        (reload_rect.min.x + reload_rect.max.x) * 0.5,
        (reload_rect.min.y + reload_rect.max.y) * 0.5,
    );
    assert!(state.source_context_menu_contains_point(&layout, &model, point));
    assert!(state.close_source_context_menu());
    assert!(!state.source_context_menu_contains_point(&layout, &model, point));
}

#[test]
/// Source context menu should expose source removal and render in the overlay pass.
fn source_context_menu_exposes_remove_action_in_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(layout.root.rect.width());
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        "source_a",
        "/tmp/source_a",
        false,
        false,
    ));
    state.open_source_context_menu_for_row(
        0,
        Point::new(layout.sidebar.min.x + 24.0, layout.sidebar.min.y + 24.0),
    );

    let remove_rect = state
        .source_context_menu_button_rect(&layout, &model, UiAction::RemoveSourceRow { index: 0 })
        .expect("remove source action button should be present");
    let point = Point::new(
        (remove_rect.min.x + remove_rect.max.x) * 0.5,
        (remove_rect.min.y + remove_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.source_context_menu_action_at_point(&layout, &model, point),
        Some(UiAction::RemoveSourceRow { index: 0 })
    );

    let frame = state.build_frame(&layout, &model);
    assert!(
        !frame
            .text_runs
            .iter()
            .any(|run| run.text == "Remove source")
    );

    let mut overlay = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut overlay);
    assert!(
        overlay
            .text_runs
            .iter()
            .any(|run| run.text == "Remove source")
    );
}

#[test]
fn top_bar_update_prefers_projected_status_and_hint_copy() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.update.status = crate::app::UpdateStatusModel::Available;
    model.update.status_label = String::from("Update available: v20.1.0");
    model.update.action_hint_label = String::from("Actions: open | install | dismiss");
    model.update.release_notes_label = String::from("Release: v20.1.0 (2026-02-01)");
    model.update.available_url = Some(String::from("https://example.invalid/release"));

    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Update available"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Actions: open"))
    );
    let controls_run = frame
        .text_runs
        .iter()
        .find(|run| run.text.contains("Actions: open"))
        .expect("combined update controls text should be present");
    assert_text_run_inside_band(controls_run, layout.top_bar_controls_row);
}

#[test]
fn tick_with_style_uses_tier_motion_speed_tokens() {
    let mut model = AppModel::default();
    model.transport_running = true;
    let compact_style = StyleTokens::for_viewport_width(820.0);
    let wide_style = StyleTokens::for_viewport_width(2300.0);

    let mut compact_state = NativeShellState::new();
    compact_state.sync_from_model(&model);
    compact_state.tick_with_style(1.0, &compact_style);

    let mut wide_state = NativeShellState::new();
    wide_state.sync_from_model(&model);
    wide_state.tick_with_style(1.0, &wide_style);

    assert!(compact_state.pulse_phase > 0.0);
    assert!(wide_state.pulse_phase > compact_state.pulse_phase);
}

#[test]
fn top_bar_volume_click_maps_to_set_volume_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let controls = top_bar_controls_layout(&layout, style_for_layout(&layout).sizing);
    assert!(controls.active);
    let point = Point::new(
        controls.volume_meter.min.x + (controls.volume_meter.width() * 0.75),
        controls.volume_meter.min.y + (controls.volume_meter.height() * 0.5),
    );
    let action = state
        .top_bar_volume_action_at_point(&layout, point)
        .expect("volume click should produce action");
    assert_eq!(action, UiAction::SetVolume { value_milli: 750 });
}

#[test]
fn top_bar_options_click_maps_to_open_options_menu_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let controls = top_bar_controls_layout(&layout, style_for_layout(&layout).sizing);
    assert!(controls.active);
    let point = Point::new(
        controls.options_label.min.x + (controls.options_label.width() * 0.5),
        controls.options_label.min.y + (controls.options_label.height() * 0.5),
    );
    let action = state
        .top_bar_options_action_at_point(&layout, point)
        .expect("options click should produce action");
    assert_eq!(action, UiAction::OpenOptionsMenu);
}

#[test]
fn top_bar_volume_drag_clamps_beyond_meter_bounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let controls = top_bar_controls_layout(&layout, style_for_layout(&layout).sizing);
    assert!(controls.active);
    let left_action = state
        .top_bar_volume_drag_action(
            &layout,
            Point::new(
                controls.volume_meter.min.x - 40.0,
                controls.volume_meter.min.y,
            ),
        )
        .expect("left drag action");
    let right_action = state
        .top_bar_volume_drag_action(
            &layout,
            Point::new(
                controls.volume_meter.max.x + 40.0,
                controls.volume_meter.min.y,
            ),
        )
        .expect("right drag action");
    assert_eq!(left_action, UiAction::SetVolume { value_milli: 0 });
    assert_eq!(right_action, UiAction::SetVolume { value_milli: 1000 });
}
