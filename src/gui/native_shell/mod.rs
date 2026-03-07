//! Backend-neutral native shell model used by the Vello runtime.
//!
//! The design mirrors a retained view tree (inspired by Xilem): build a
//! deterministic layout tree, run hit testing against that tree, then derive
//! backend-neutral paint primitives (shapes + text runs).

mod layout;
mod layout_adapter;
mod layout_runtime;
mod paint;
#[cfg(test)]
mod shots;
mod state;
mod style;

pub(crate) use layout::ShellLayout;
pub(crate) use layout::ShellNodeKind;
pub(crate) use layout_runtime::{ShellLayoutDirtyKind, ShellLayoutRuntime};
pub(crate) use paint::{NativeViewFrame, Primitive, TextAlign, TextRun};
pub(crate) use state::{
    ChromeMotionOverlayFingerprint, CursorMoveEffect, NativeShellState, StateOverlayFingerprint,
    StaticFrameSegment, StaticFrameSegments, TextFieldVisualState,
    WaveformMotionOverlayFingerprint,
};
pub(crate) use style::StyleTokens;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::{
        input::KeyCode,
        types::{Point, Vector2},
    };

    fn canonical_shell_model() -> crate::app::AppModel {
        let mut model = crate::app::AppModel::default();
        model.title = String::from("Sempal Native");
        model.backend_label = String::from("radiant/native_vello");
        model.transport_running = true;
        model.status.left = String::from("Indexing complete");
        model.status.center = String::from("rows: 48 | selected: 3");
        model.status.right = String::from("col: 2/3");
        model.sources.search_query = String::from("drum");
        model.sources.folder_search_query = String::from("kicks");
        model.sources.folder_recovery.in_progress = false;
        model.sources.folder_recovery.entry_count = 12;
        model.sources.folder_actions.can_create_folder = true;
        model.sources.folder_actions.can_create_folder_at_root = true;
        model.sources.folder_actions.can_rename_folder = true;
        model.sources.folder_actions.can_delete_folder = true;
        model.sources.folder_actions.can_clear_recovery_log = true;
        for index in 0..10 {
            model.sources.rows.push(crate::app::SourceRowModel::new(
                format!("source_{index:02}"),
                format!("/samples/source_{index:02}"),
                index == 2,
                index == 5,
            ));
        }
        for index in 0..14 {
            model
                .sources
                .folder_rows
                .push(crate::app::FolderRowModel::new(
                    format!("folder_{index:02}"),
                    String::new(),
                    index % 3,
                    index == 1,
                    index == 3,
                    index == 0,
                    true,
                    true,
                ));
        }
        for index in 0..36 {
            model.browser.rows.push(crate::app::BrowserRowModel::new(
                index,
                format!("row_{index:02}.wav"),
                index % 3,
                index % 8 == 0,
                index == 5,
            ));
        }
        model.browser.visible_count = model.browser.rows.len();
        model.browser.selected_path_count = 3;
        model.browser.search_query = String::from("kick");
        model.browser_chrome.search_prefix_label = String::from("Find");
        model.browser_chrome.sort_prefix_label = String::from("Order");
        model.browser_chrome.sort_order_label = String::from("List order");
        model.browser_chrome.item_count_label = String::from("36 items");
        model.waveform_chrome.transport_hint = String::from("Loop engaged");
        model.waveform.loaded_label = Some(String::from("Kick-Loop-01.wav"));
        model.waveform.cursor_milli = Some(345);
        model.waveform.playhead_milli = Some(512);
        model.waveform.selection_milli = Some(crate::app::NormalizedRangeModel::new(200, 680));
        model.waveform.loop_enabled = true;
        model.waveform.view_start_milli = 100;
        model.waveform.view_end_milli = 900;
        model
    }

    #[test]
    fn layout_exposes_non_overlapping_columns() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.columns[0].max.x <= layout.columns[1].min.x);
        assert!(layout.columns[1].max.x <= layout.columns[2].min.x);
        assert!(layout.columns.iter().all(|column| column.width() > 40.0));
    }

    #[test]
    fn hit_test_prefers_column_node_inside_content() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let center = Point::new(
            (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
            (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
        );
        assert_eq!(
            layout.hit_test(center),
            Some(layout::ShellNodeKind::BrowserTable)
        );
    }

    #[test]
    fn primary_click_selects_clicked_column() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let point = Point::new(
            (layout.columns[2].min.x + layout.columns[2].max.x) * 0.5,
            (layout.columns[2].min.y + layout.columns[2].max.y) * 0.5,
        );
        assert!(state.handle_primary_click(&layout, point));
        let frame = state.build_frame(&layout, &crate::app::AppModel::default());
        assert!(frame.primitives.len() > 10);
        assert!(!frame.text_runs.is_empty());
    }

    #[test]
    fn arrow_keys_wrap_selection() {
        let mut state = NativeShellState::new();
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowLeft));
    }

    #[test]
    fn browser_row_hit_test_resolves_visible_row() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model
            .browser
            .rows
            .push(crate::app::BrowserRowModel::new(7, "kick", 0, false, true));
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let row_center_y = layout.browser_rows.min.y + (style.sizing.browser_row_height * 0.5);
        let point = Point::new(
            (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
            row_center_y,
        );
        assert_eq!(state.browser_row_at_point(&layout, &model, point), Some(7));
        state.sync_from_model(&model);
        let frame = state.build_frame(&layout, &model);
        assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
    }

    #[test]
    fn compact_layout_keeps_tight_header_and_footer_bands() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.top_bar.height() <= 40.0);
        assert!(layout.status_bar.height() <= 24.0);
        assert!(layout.waveform_card.height() >= 120.0);
    }

    #[test]
    fn compact_layout_preserves_content_on_narrow_viewports() {
        let layout = ShellLayout::build(Vector2::new(820.0, 520.0));
        assert!(layout.sidebar.width() >= 160.0);
        assert!(layout.content.width() >= 200.0);
        assert!(layout.columns.iter().all(|column| column.width() >= 40.0));
    }

    #[test]
    fn classic_reference_viewport_matches_dense_geometry_contract() {
        let viewport = Vector2::new(1440.0, 810.0);
        let style = style::StyleTokens::for_viewport_width(viewport.x);
        let layout = ShellLayout::build(viewport);
        let snapshot = layout.contract_snapshot(&style);

        assert!(snapshot.sidebar_width >= style.sizing.sidebar_min_width - 1.0);
        assert!(snapshot.sidebar_width <= style.sizing.sidebar_max_width + 1.0);
        assert!(snapshot.waveform_height >= style.sizing.waveform_min_height - 1.0);
        assert!(snapshot.waveform_height <= style.sizing.waveform_max_height + 1.0);
        assert!(snapshot.browser_row_capacity >= 22);
        assert!(snapshot.top_bar_height <= 34.0);
        assert!(snapshot.status_bar_height <= 20.0);
    }

    #[test]
    fn layout_snapshot_clamps_to_tokenized_min_viewport() {
        let style = style::StyleTokens::for_viewport_width(0.0);
        let layout = ShellLayout::build_with_style(Vector2::new(1.0, 1.0), &style);
        let snapshot = layout.contract_snapshot(&style);
        assert_eq!(snapshot.viewport_width, style.sizing.min_viewport_width);
        assert_eq!(snapshot.viewport_height, style.sizing.min_viewport_height);
    }

    #[test]
    fn scaled_layout_preserves_scale_ratio_for_rebuild() {
        let viewport = Vector2::new(1280.0, 720.0);
        let scaled_style = style::StyleTokens::for_viewport_with_scale(viewport.x, 1.6);
        let base_style = style::StyleTokens::for_viewport_width(viewport.x);
        let layout = ShellLayout::build_with_style(viewport, &scaled_style);

        let rebuilt_style =
            style::StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);

        let expected_scale = scaled_style.sizing.font_body / base_style.sizing.font_body;
        let observed_scale = rebuilt_style.sizing.font_body / base_style.sizing.font_body;

        assert!((layout.ui_scale - 1.6).abs() < 0.0001);
        assert!((expected_scale - observed_scale).abs() < 0.0001);
    }

    #[test]
    fn viewport_tier_sizing_changes_row_density() {
        let narrow = style::StyleTokens::for_viewport_width(820.0);
        let wide = style::StyleTokens::for_viewport_width(2300.0);
        assert!(narrow.sizing.browser_row_height < wide.sizing.browser_row_height);
        assert!(narrow.sizing.source_row_height < wide.sizing.source_row_height);
    }

    #[test]
    fn visual_density_snapshot_scales_across_tiers() {
        let compact_viewport = Vector2::new(820.0, 520.0);
        let standard_viewport = Vector2::new(1280.0, 720.0);
        let wide_viewport = Vector2::new(2300.0, 1080.0);

        let compact_style = style::StyleTokens::for_viewport_width(compact_viewport.x);
        let standard_style = style::StyleTokens::for_viewport_width(standard_viewport.x);
        let wide_style = style::StyleTokens::for_viewport_width(wide_viewport.x);

        let compact = ShellLayout::build(compact_viewport).contract_snapshot(&compact_style);
        let standard = ShellLayout::build(standard_viewport).contract_snapshot(&standard_style);
        let wide = ShellLayout::build(wide_viewport).contract_snapshot(&wide_style);

        assert!(compact.top_bar_height >= standard.top_bar_height);
        assert!(wide.top_bar_height >= standard.top_bar_height);
        assert!(compact.status_bar_height >= standard.status_bar_height);
        assert!(wide.status_bar_height >= standard.status_bar_height);
        assert!(compact.browser_row_capacity <= standard.browser_row_capacity);
        assert!(standard.browser_row_capacity <= wide.browser_row_capacity);
    }

    #[test]
    fn layout_bands_stay_within_panel_bounds() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert_eq!(layout.top_bar_title_row, layout.top_bar);
        assert_eq!(layout.top_bar_controls_row, layout.top_bar);
        assert!(layout.sidebar_header.max.y <= layout.sidebar_rows.min.y);
        assert!(layout.sidebar_rows.max.y <= layout.sidebar_footer.min.y);
        assert_eq!(layout.waveform_header.max.y, layout.waveform_plot.min.y);
        assert!(layout.browser_tabs.max.y <= layout.browser_toolbar.min.y);
        assert!(layout.browser_toolbar.max.y <= layout.browser_table_header.min.y);
        assert!(layout.browser_table_header.max.y <= layout.browser_rows.min.y);
        assert!(layout.browser_rows.max.y <= layout.browser_footer.min.y);
        for index in 0..3 {
            assert!(layout.column_headers[index].max.y <= layout.column_rows[index].min.y);
            assert!(layout.column_rows[index].min.x >= layout.columns[index].min.x);
            assert!(layout.column_rows[index].max.x <= layout.columns[index].max.x);
        }
    }

    #[test]
    fn major_panels_share_edges_without_gap() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert_eq!(layout.top_bar.max.y, layout.sidebar.min.y);
        assert_eq!(layout.top_bar.max.y, layout.content.min.y);
        assert_eq!(layout.sidebar.max.x, layout.content.min.x);
        assert_eq!(layout.waveform_card.max.y, layout.browser_panel.min.y);
        assert_eq!(layout.sidebar.max.y, layout.status_bar.min.y);
        assert_eq!(layout.browser_panel.max.y, layout.status_bar.min.y);
    }

    #[test]
    fn browser_bands_fill_browser_panel_width_without_inner_gutters() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert_eq!(layout.browser_tabs.min.x, layout.browser_panel.min.x);
        assert_eq!(layout.browser_tabs.max.x, layout.browser_panel.max.x);
        assert_eq!(layout.browser_toolbar.min.x, layout.browser_panel.min.x);
        assert_eq!(layout.browser_toolbar.max.x, layout.browser_panel.max.x);
        assert_eq!(
            layout.browser_table_header.min.x,
            layout.browser_panel.min.x
        );
        assert_eq!(
            layout.browser_table_header.max.x,
            layout.browser_panel.max.x
        );
        assert_eq!(layout.browser_rows.min.x, layout.browser_panel.min.x);
        assert_eq!(layout.browser_rows.max.x, layout.browser_panel.max.x);
        assert_eq!(layout.browser_footer.min.x, layout.browser_panel.min.x);
        assert_eq!(layout.browser_footer.max.x, layout.browser_panel.max.x);
    }

    #[test]
    fn top_bar_clusters_stay_ordered_and_inside_bar() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.top_bar_title_cluster.min.x >= layout.top_bar.min.x);
        assert!(layout.top_bar_title_cluster.max.y <= layout.top_bar_title_row.max.y);
        assert!(layout.top_bar_action_cluster.min.x >= layout.top_bar.min.x);
        assert!(layout.top_bar_action_cluster.max.y <= layout.top_bar_title_row.max.y);
        assert!(layout.top_bar_title_cluster.max.x <= layout.top_bar_action_cluster.min.x);
    }

    #[test]
    fn top_bar_clusters_reserve_minimum_title_and_action_widths() {
        let viewport = Vector2::new(1280.0, 720.0);
        let tokens = style::StyleTokens::for_viewport_width(viewport.x);
        let layout = ShellLayout::build(viewport);
        assert!(
            layout.top_bar_action_cluster.width()
                >= tokens.sizing.top_bar_action_cluster_min_width - 1.0
        );
        assert!(
            layout.top_bar_title_cluster.width()
                >= tokens.sizing.top_bar_action_cluster_title_reserve_width - 1.0
        );
    }

    #[test]
    fn status_segments_remain_non_overlapping_and_bounded() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.status_left_segment.min.x >= layout.status_bar.min.x);
        assert!(layout.status_right_segment.max.x <= layout.status_bar.max.x);
        assert!(layout.status_left_segment.max.x <= layout.status_center_segment.min.x);
        assert!(layout.status_center_segment.max.x <= layout.status_right_segment.min.x);
        assert!(layout.status_left_segment.max.y <= layout.status_bar.max.y);
        assert!(layout.status_center_segment.max.y <= layout.status_bar.max.y);
        assert!(layout.status_right_segment.max.y <= layout.status_bar.max.y);
    }

    #[test]
    fn layout_uses_tokenized_shell_heights() {
        let width = 1280.0;
        let height = 720.0;
        let layout = ShellLayout::build(Vector2::new(width, height));
        let tokens = style::StyleTokens::for_viewport_width(width);
        assert!((layout.top_bar.height() - tokens.sizing.top_bar_height).abs() < 0.001);
        assert!((layout.status_bar.height() - tokens.sizing.status_bar_height).abs() < 0.001);
    }

    #[test]
    fn browser_header_band_can_fit_single_metadata_line_across_tiers() {
        for viewport in [
            Vector2::new(820.0, 520.0),
            Vector2::new(1280.0, 720.0),
            Vector2::new(2300.0, 1080.0),
        ] {
            let tokens = style::StyleTokens::for_viewport_width(viewport.x);
            let layout = ShellLayout::build(viewport);
            let centered_y = layout.browser_table_header.min.y
                + ((layout.browser_table_header.height() - tokens.sizing.font_meta).max(0.0) * 0.5);
            let top =
                centered_y.max(layout.browser_table_header.min.y + tokens.sizing.text_inset_y);
            assert!(top + tokens.sizing.font_meta <= layout.browser_table_header.max.y + 0.5);
        }
    }

    #[test]
    fn toolbar_hit_test_focuses_browser_search() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let model = crate::app::AppModel::default();
        let search_field = state
            .browser_search_field_rect(&layout, &model)
            .expect("browser search field should be present");
        let point = Point::new(
            (search_field.min.x + search_field.max.x) * 0.5,
            (search_field.min.y + search_field.max.y) * 0.5,
        );
        assert_eq!(
            state.browser_action_at_point(&layout, &model, point),
            Some(crate::app::UiAction::FocusBrowserSearch)
        );
    }

    #[test]
    fn toolbar_hit_test_toggles_browser_rating_filter_chip() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let model = crate::app::AppModel::default();
        let chip = state
            .browser_rating_filter_chip_rect(&layout, &model, 3)
            .expect("keep-3 rating filter chip should be present");
        let point = Point::new(
            (chip.min.x + chip.max.x) * 0.5,
            (chip.min.y + chip.max.y) * 0.5,
        );
        assert_eq!(
            state.browser_action_at_point(&layout, &model, point),
            Some(crate::app::UiAction::ToggleBrowserRatingFilter { level: 3 })
        );
    }

    #[test]
    fn toolbar_hit_test_ignores_empty_right_host_area() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let model = crate::app::AppModel::default();
        let point = Point::new(
            layout.browser_toolbar.max.x - 12.0,
            (layout.browser_toolbar.min.y + layout.browser_toolbar.max.y) * 0.5,
        );
        assert_eq!(state.browser_action_at_point(&layout, &model, point), None);
    }

    #[test]
    fn browser_toolbar_exposes_no_column_chip_hit_targets() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.columns[2].item_count = 42;
        assert!(state.browser_column_chip_rect(&layout, &model, 2).is_none());
    }

    #[test]
    fn waveform_toolbar_hit_test_emits_transport_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let model = crate::app::AppModel::default();
        let play = state
            .waveform_toolbar_button_rect(&layout, &model, "Play")
            .expect("play waveform toolbar button should be present");
        let point = Point::new(
            (play.min.x + play.max.x) * 0.5,
            (play.min.y + play.max.y) * 0.5,
        );
        assert_eq!(
            state.waveform_toolbar_action_at_point(&layout, &model, point),
            Some(crate::app::UiAction::ToggleTransport)
        );
    }

    #[test]
    fn waveform_toolbar_hit_test_emits_loop_toggle_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.waveform.loop_enabled = true;
        let loop_button = state
            .waveform_toolbar_button_rect(&layout, &model, "Loop")
            .expect("loop waveform toolbar button should be present");
        let point = Point::new(
            (loop_button.min.x + loop_button.max.x) * 0.5,
            (loop_button.min.y + loop_button.max.y) * 0.5,
        );
        assert_eq!(
            state.waveform_toolbar_action_at_point(&layout, &model, point),
            Some(crate::app::UiAction::ToggleLoopPlayback)
        );
    }

    #[test]
    /// Waveform toolbar BPM value widget should expose a hit-test area for editing.
    fn waveform_toolbar_bpm_value_widget_exposes_input_hit_target() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.waveform.tempo_label = Some(String::from("128.0 BPM"));
        let bpm_value = state
            .waveform_toolbar_button_rect(&layout, &model, "BPM Value")
            .expect("bpm value waveform toolbar widget should be present");
        let point = Point::new(
            (bpm_value.min.x + bpm_value.max.x) * 0.5,
            (bpm_value.min.y + bpm_value.max.y) * 0.5,
        );
        assert!(state.waveform_bpm_input_at_point(&layout, &model, point));
    }

    #[test]
    fn prompt_hit_test_emits_confirm_and_cancel() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.confirm_prompt.visible = true;
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let dialog = {
            let sizing = style.sizing;
            let width = sizing
                .prompt_width
                .min(layout.content.width() - (sizing.overlay_padding * 2.0))
                .max(260.0);
            let height = sizing
                .prompt_min_height
                .min(layout.content.height() - (sizing.overlay_padding * 2.0))
                .max(108.0);
            let x = layout.content.min.x + (layout.content.width() - width).max(0.0) * 0.5;
            let y = layout.content.min.y + (layout.content.height() - height).max(0.0) * 0.35;
            crate::gui::types::Rect::from_min_max(
                Point::new(x, y),
                Point::new(x + width, y + height),
            )
        };
        let cancel = {
            let sizing = style.sizing;
            crate::gui::types::Rect::from_min_max(
                Point::new(
                    dialog.max.x - sizing.overlay_button_width - sizing.text_inset_x,
                    dialog.max.y - sizing.overlay_button_height - sizing.text_inset_y,
                ),
                Point::new(
                    dialog.max.x - sizing.text_inset_x,
                    dialog.max.y - sizing.text_inset_y,
                ),
            )
        };
        let confirm = {
            let sizing = style.sizing;
            crate::gui::types::Rect::from_min_max(
                Point::new(
                    cancel.min.x - sizing.overlay_button_width - sizing.action_button_gap,
                    cancel.min.y,
                ),
                Point::new(cancel.min.x - sizing.action_button_gap, cancel.max.y),
            )
        };
        let confirm_point = Point::new(
            (confirm.min.x + confirm.max.x) * 0.5,
            (confirm.min.y + confirm.max.y) * 0.5,
        );
        let cancel_point = Point::new(
            (cancel.min.x + cancel.max.x) * 0.5,
            (cancel.min.y + cancel.max.y) * 0.5,
        );
        assert_eq!(
            state.prompt_action_at_point(&layout, &model, confirm_point),
            Some(crate::app::UiAction::ConfirmPrompt)
        );
        assert_eq!(
            state.prompt_action_at_point(&layout, &model, cancel_point),
            Some(crate::app::UiAction::CancelPrompt)
        );
    }

    #[test]
    fn prompt_input_hit_test_resolves_text_entry_rect() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.confirm_prompt.visible = true;
        model.confirm_prompt.input_value = Some(String::from("kicks"));
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let sizing = style.sizing;
        let dialog = {
            let width = sizing
                .prompt_width
                .min(layout.content.width() - (sizing.overlay_padding * 2.0))
                .max(260.0);
            let height = sizing
                .prompt_min_height
                .min(layout.content.height() - (sizing.overlay_padding * 2.0))
                .max(108.0);
            let x = layout.content.min.x + (layout.content.width() - width).max(0.0) * 0.5;
            let y = layout.content.min.y + (layout.content.height() - height).max(0.0) * 0.35;
            crate::gui::types::Rect::from_min_max(
                Point::new(x, y),
                Point::new(x + width, y + height),
            )
        };
        let input_y = dialog.min.y
            + sizing.text_inset_y
            + sizing.font_title
            + sizing.font_meta
            + (sizing.text_row_gap * 4.0);
        let point = Point::new(dialog.min.x + 20.0, input_y + 8.0);
        assert!(state.prompt_input_at_point(&layout, &model, point));
    }

    #[test]
    fn prompt_confirm_hit_test_is_blocked_when_input_error_is_present() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.confirm_prompt.visible = true;
        model.confirm_prompt.input_value = Some(String::from("bad/name"));
        model.confirm_prompt.input_error =
            Some(String::from("Folder name cannot contain path separators"));
        let style = style::StyleTokens::for_viewport_width(layout.root.rect.width());
        let sizing = style.sizing;
        let dialog = {
            let width = sizing
                .prompt_width
                .min(layout.content.width() - (sizing.overlay_padding * 2.0))
                .max(260.0);
            let height = sizing
                .prompt_min_height
                .min(layout.content.height() - (sizing.overlay_padding * 2.0))
                .max(108.0);
            let x = layout.content.min.x + (layout.content.width() - width).max(0.0) * 0.5;
            let y = layout.content.min.y + (layout.content.height() - height).max(0.0) * 0.35;
            crate::gui::types::Rect::from_min_max(
                Point::new(x, y),
                Point::new(x + width, y + height),
            )
        };
        let cancel = crate::gui::types::Rect::from_min_max(
            Point::new(
                dialog.max.x - sizing.overlay_button_width - sizing.text_inset_x,
                dialog.max.y - sizing.overlay_button_height - sizing.text_inset_y,
            ),
            Point::new(
                dialog.max.x - sizing.text_inset_x,
                dialog.max.y - sizing.text_inset_y,
            ),
        );
        let confirm = crate::gui::types::Rect::from_min_max(
            Point::new(
                cancel.min.x - sizing.overlay_button_width - sizing.action_button_gap,
                cancel.min.y,
            ),
            Point::new(cancel.min.x - sizing.action_button_gap, cancel.max.y),
        );
        let confirm_point = Point::new(
            (confirm.min.x + confirm.max.x) * 0.5,
            (confirm.min.y + confirm.max.y) * 0.5,
        );
        assert_eq!(
            state.prompt_action_at_point(&layout, &model, confirm_point),
            None
        );
    }

    #[test]
    fn source_action_hit_test_emits_folder_action() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.sources.folder_actions.can_delete_folder = true;
        let button = state
            .source_action_button_rect(&layout, &model, crate::app::UiAction::DeleteFocusedFolder)
            .expect("delete action button should be present");
        let point = Point::new(
            (button.min.x + button.max.x) * 0.5,
            (button.min.y + button.max.y) * 0.5,
        );
        assert_eq!(
            state.source_action_at_point(&layout, &model, point),
            Some(crate::app::UiAction::DeleteFocusedFolder)
        );
    }

    #[test]
    fn source_action_hit_test_ignores_disabled_button() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.sources.folder_actions.can_delete_folder = false;
        let button = state
            .source_action_button_rect(&layout, &model, crate::app::UiAction::DeleteFocusedFolder)
            .expect("delete action button should be present");
        let point = Point::new(
            (button.min.x + button.max.x) * 0.5,
            (button.min.y + button.max.y) * 0.5,
        );
        assert_eq!(state.source_action_at_point(&layout, &model, point), None);
    }

    #[test]
    fn folder_row_hit_test_resolves_rendered_folder_row() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model
            .sources
            .folder_rows
            .push(crate::app::FolderRowModel::new(
                "Drums", "Drums", 0, false, true, false, true, true,
            ));
        let folder_rects = state.rendered_folder_row_rects(&layout, &model);
        assert_eq!(folder_rects.len(), 1);
        let folder_rect = folder_rects[0];
        let point = Point::new(
            (folder_rect.min.x + folder_rect.max.x) * 0.5,
            (folder_rect.min.y + folder_rect.max.y) * 0.5,
        );
        assert_eq!(state.folder_row_at_point(&layout, &model, point), Some(0));
    }

    #[test]
    fn folder_row_hit_test_survives_source_row_cache_priming() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.sources.rows.push(crate::app::SourceRowModel::new(
            "Pack", "pack", false, false,
        ));
        model
            .sources
            .folder_rows
            .push(crate::app::FolderRowModel::new(
                "Drums", "Drums", 0, false, true, false, true, true,
            ));

        let source_rects = state.rendered_source_row_rects(&layout, &model);
        assert_eq!(source_rects.len(), 1);
        let source_point = Point::new(
            (source_rects[0].min.x + source_rects[0].max.x) * 0.5,
            (source_rects[0].min.y + source_rects[0].max.y) * 0.5,
        );
        assert_eq!(
            state.source_row_at_point(&layout, &model, source_point),
            Some(0)
        );

        let folder_rects = state.rendered_folder_row_rects(&layout, &model);
        assert_eq!(folder_rects.len(), 1);
        let folder_point = Point::new(
            (folder_rects[0].min.x + folder_rects[0].max.x) * 0.5,
            (folder_rects[0].min.y + folder_rects[0].max.y) * 0.5,
        );
        assert_eq!(
            state.folder_row_at_point(&layout, &model, folder_point),
            Some(0)
        );
    }

    #[test]
    fn folder_rows_fill_sidebar_width_and_touch_without_gap() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        for index in 0..3 {
            model
                .sources
                .folder_rows
                .push(crate::app::FolderRowModel::new(
                    format!("Folder {index}"),
                    String::new(),
                    0,
                    false,
                    index == 1,
                    false,
                    true,
                    true,
                ));
        }

        let folder_rects = state.rendered_folder_row_rects(&layout, &model);
        assert_eq!(folder_rects.len(), 3);
        assert_eq!(folder_rects[0].min.x, layout.sidebar_rows.min.x);
        assert_eq!(folder_rects[0].max.x, layout.sidebar_rows.max.x);
        assert_eq!(folder_rects[0].max.y, folder_rects[1].min.y);
        assert_eq!(folder_rects[1].max.y, folder_rects[2].min.y);
    }

    #[test]
    fn focused_rows_do_not_enable_idle_animation_when_transport_is_stopped() {
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.transport_running = false;
        model
            .browser
            .rows
            .push(crate::app::BrowserRowModel::new(0, "kick", 1, false, true));
        state.sync_from_model(&model);
        state.sync_from_model(&model);
        assert!(!state.needs_animation());

        let mut idle_model = crate::app::AppModel::default();
        idle_model.transport_running = false;
        let mut playing_model = crate::app::AppModel::default();
        playing_model.transport_running = true;
        state.sync_from_model(&playing_model);
        assert!(state.needs_animation());
        state.sync_from_model(&idle_model);
        assert!(!state.needs_animation());
    }

    #[test]
    fn long_browser_labels_are_truncated_with_ellipsis() {
        let layout = ShellLayout::build(Vector2::new(620.0, 420.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        model.browser.rows.push(crate::app::BrowserRowModel::new(
            0,
            "this_is_a_very_long_browser_row_label_that_should_truncate_in_native_shell_rendering_and_is_intentionally_longer_than_any_practical_row_width_even_on_narrow_compact_views.wav",
            1,
            false,
            false,
        ));
        state.sync_from_model(&model);
        let frame = state.build_frame(&layout, &model);
        let truncated = frame
            .text_runs
            .iter()
            .find(|run| run.text.starts_with("this_is_a"))
            .map(|run| run.text.as_str())
            .unwrap_or_default();
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn wide_viewport_renders_more_browser_rows_than_narrow_viewport() {
        let narrow_layout = ShellLayout::build(Vector2::new(820.0, 520.0));
        let wide_layout = ShellLayout::build(Vector2::new(2300.0, 1080.0));
        let mut state = NativeShellState::new();
        let mut model = crate::app::AppModel::default();
        for index in 0..40 {
            model.browser.rows.push(crate::app::BrowserRowModel::new(
                index,
                format!("row_{index:02}"),
                1,
                false,
                false,
            ));
        }
        state.sync_from_model(&model);
        let narrow_frame = state.build_frame(&narrow_layout, &model);
        let wide_frame = state.build_frame(&wide_layout, &model);
        let narrow_rows = narrow_frame
            .text_runs
            .iter()
            .filter(|run| run.text.starts_with("row_"))
            .count();
        let wide_rows = wide_frame
            .text_runs
            .iter()
            .filter(|run| run.text.starts_with("row_"))
            .count();
        assert!(wide_rows > narrow_rows);
    }

    #[test]
    fn canonical_frame_rebuild_is_deterministic_across_tiers() {
        let mut state = NativeShellState::new();
        let model = canonical_shell_model();
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
            assert!(!frame_a.primitives.is_empty());
            assert!(!frame_a.text_runs.is_empty());
        }
    }

    #[test]
    fn canonical_frame_contains_expected_sidebar_and_status_contract_text() {
        let mut state = NativeShellState::new();
        let model = canonical_shell_model();
        state.sync_from_model(&model);
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let frame = state.build_frame(&layout, &model);
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Folders ("))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("entries"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("rows: 48"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("col: 2/3"))
        );
        assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("36 items"))
        );
        assert!(
            frame
                .text_runs
                .iter()
                .any(|run| run.text.contains("Loop engaged"))
        );
    }
}
